use actix_web::{web, App, HttpResponse, HttpServer};
use std::sync::Arc;

use server_monitoring::app_state::AppState;
use server_monitoring::cache::api_key_cache::ApiKeyCache;
use server_monitoring::cache::ingest_quota_tracker::IngestQuotaTracker;
use server_monitoring::cache::tenant_config_cache::TenantConfigCache;
use server_monitoring::config::database::{create_sea_orm_db, MongoConnection};
use server_monitoring::config::messaging::RabbitMqConnection;
use server_monitoring::config::settings::AppConfig;
use server_monitoring::config::telemetry;
use server_monitoring::messaging::producer::EventProducer;
use server_monitoring::middleware::request_logger::RequestLogger;
use server_monitoring::repository::api_key_repo::MongoApiKeyRepository;
use server_monitoring::repository::client_repo::MongoClientRepository;
use server_monitoring::repository::metrics_repo::SeaOrmMetricsRepository;
use server_monitoring::repository::tenant_config_repo::SeaOrmTenantConfigRepository;
use server_monitoring::repository::user_repo::MongoUserRepository;
use server_monitoring::resilience::circuit_breaker::CircuitBreaker;
use server_monitoring::resilience::retry::RetryStrategy;
use server_monitoring::router::routes;
use server_monitoring::service::analytics::AnalyticsService;
use server_monitoring::service::auth::AuthService;
use server_monitoring::service::client::ClientService;
use server_monitoring::service::ingest::IngestService;
use server_monitoring::service::tenant_config::TenantConfigService;
use server_monitoring::util::response::ResponseFormatter;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load .env (no-op if file is absent)
    dotenvy::dotenv().ok();

    // Load typed config — panics with a clear message on missing required vars
    let config = AppConfig::from_env();

    telemetry::init_telemetry(&config.environment);

    tracing::info!(
        port = config.port,
        env  = %config.environment,
        "Starting Server Monitoring System (Rust)"
    );

    // ── Database connections ──────────────────────────────────────────────────

    let mut mongo_conn = MongoConnection::new(config.mongo.clone());
    let db = mongo_conn
        .connect()
        .await
        .expect("Failed to connect to MongoDB");

    let sea_db = create_sea_orm_db(&config.postgres_connection_string())
        .await
        .unwrap_or_else(|e| {
            panic!(
                "Failed to connect to PostgreSQL ({}): {}\n\
                 Check PG_HOST, PG_PORT, PG_DATABASE, PG_USER, PG_PASSWORD in your .env file.",
                config.postgres_connection_string(),
                e
            )
        });

    // ── Message broker ────────────────────────────────────────────────────────

    let mut rmq_conn = RabbitMqConnection::new(config.rabbitmq.clone());
    let channel = rmq_conn
        .connect()
        .await
        .expect("Failed to connect to RabbitMQ");

    // ── Repository layer (trait objects for DI) ───────────────────────────────

    let user_repo: Arc<dyn server_monitoring::repository::user_repo::UserRepository> =
        Arc::new(MongoUserRepository::new(&db));

    let client_repo: Arc<dyn server_monitoring::repository::client_repo::ClientRepository> =
        Arc::new(MongoClientRepository::new(&db));

    let api_key_repo: Arc<dyn server_monitoring::repository::api_key_repo::ApiKeyRepository> =
        Arc::new(MongoApiKeyRepository::new(&db));

    let metrics_repo: Arc<dyn server_monitoring::repository::metrics_repo::MetricsRepository> =
        Arc::new(SeaOrmMetricsRepository::new(sea_db.clone()));

    let tenant_config_repo: Arc<
        dyn server_monitoring::repository::tenant_config_repo::TenantConfigRepository,
    > = Arc::new(SeaOrmTenantConfigRepository::new(sea_db.clone()));

    // ── Caches ────────────────────────────────────────────────────────────────

    let api_key_cache = Arc::new(ApiKeyCache::new(
        config.api_key_cache.ttl_secs,
        config.api_key_cache.max_capacity,
    ));

    let tenant_config_cache = Arc::new(TenantConfigCache::new(
        config.tenant_config_cache.ttl_secs,
        config.tenant_config_cache.max_capacity,
    ));

    let quota_tracker = Arc::new(IngestQuotaTracker::new());

    // ── Service layer ─────────────────────────────────────────────────────────

    let auth_service = AuthService::new(user_repo.clone(), config.clone());

    let client_service = ClientService::new(client_repo, api_key_repo, user_repo);

    let analytics_service = AnalyticsService::new(metrics_repo.clone());

    let tenant_config_service = TenantConfigService::new(
        tenant_config_repo,
        tenant_config_cache,
    );

    // ── Resilience primitives ─────────────────────────────────────────────────

    let circuit_breaker = Arc::new(CircuitBreaker::new(
        config.resilience.cb_failure_threshold,
        config.resilience.cb_cooldown_ms,
        config.resilience.cb_half_open_attempts,
    ));
    let retry_strategy = Arc::new(RetryStrategy::new(
        config.rabbitmq.retry_attempts,
        config.rabbitmq.retry_delay,
        config.resilience.retry_max_delay_ms,
        config.resilience.retry_jitter_factor,
    ));
    let event_producer = EventProducer::new(
        channel,
        config.rabbitmq.clone(),
        circuit_breaker.clone(),
        retry_strategy,
    );

    let ingest_service = IngestService::new(event_producer, quota_tracker.clone());

    // ── Dependency injection container ────────────────────────────────────────

    let app_state = web::Data::new(AppState::new(
        auth_service,
        client_service,
        analytics_service,
        ingest_service,
        tenant_config_service,
        api_key_cache,
        quota_tracker,
        sea_db,
        config.clone(),
        circuit_breaker,
    ));

    let port         = config.port;
    let config_clone = config.clone();

    // ── Background tasks ──────────────────────────────────────────────────────

    // F4: Prune stale IngestQuotaTracker entries every 10 minutes.
    // Prevents unbounded memory growth for high-cardinality tenants.
    {
        let tracker_bg = app_state.quota_tracker.clone();
        tokio::spawn(async move {
            let interval = std::time::Duration::from_secs(10 * 60); // 10 min
            loop {
                tokio::time::sleep(interval).await;
                tracker_bg.prune_stale_entries();
                tracing::debug!("IngestQuotaTracker: pruned stale client entries");
            }
        });
    }

    tracing::info!("Server starting on port {}", port);

    // ── HTTP server ───────────────────────────────────────────────────────────

    let server = HttpServer::new(move || {
        let cors = actix_cors::Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .supports_credentials();

        App::new()
            .app_data(app_state.clone())
            .wrap(cors)
            .wrap(RequestLogger)
            .configure(|cfg| routes::configure(cfg, &config_clone))
            .default_service(web::route().to(|| async {
                HttpResponse::NotFound()
                    .json(ResponseFormatter::not_found("Endpoint not found"))
            }))
    })
    .bind(format!("0.0.0.0:{port}"))?
    .shutdown_timeout(30)
    .run();

    let server_handle = server.handle();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            tracing::info!("Shutdown signal received — stopping gracefully");
            server_handle.stop(true).await;
        }
    });

    server.await
}

