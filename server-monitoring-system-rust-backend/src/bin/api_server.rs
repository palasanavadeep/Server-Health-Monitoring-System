use actix_web::{web, App, HttpResponse, HttpServer};
use std::sync::Arc;

use server_monitoring::app_state::AppState;
use server_monitoring::config::database::{create_sea_orm_db, MongoConnection};
use server_monitoring::config::messaging::RabbitMqConnection;
use server_monitoring::config::settings::AppConfig;
use server_monitoring::config::telemetry;
use server_monitoring::messaging::producer::EventProducer;
use server_monitoring::middleware::request_logger::RequestLogger;
use server_monitoring::repository::api_key_repo::MongoApiKeyRepository;
use server_monitoring::repository::client_repo::MongoClientRepository;
use server_monitoring::repository::metrics_repo::SeaOrmMetricsRepository;
use server_monitoring::repository::user_repo::MongoUserRepository;
use server_monitoring::resilience::circuit_breaker::CircuitBreaker;
use server_monitoring::resilience::retry::RetryStrategy;
use server_monitoring::router::routes;
use server_monitoring::service::analytics::AnalyticsService;
use server_monitoring::service::auth::AuthService;
use server_monitoring::service::client::ClientService;
use server_monitoring::service::ingest::IngestService;
use server_monitoring::util::response::ResponseFormatter;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables from .env file (no-op if file missing)
    dotenvy::dotenv().ok();

    // Load and validate typed configuration — panics with a clear message
    // if required env vars (PG_PASSWORD, JWT_SECRET) are not set.
    let config = AppConfig::from_env();

    // Initialize structured logging / tracing
    telemetry::init_telemetry(&config.environment);

    tracing::info!(
        port = config.port,
        env = %config.environment,
        "Starting Server Monitoring System (Rust)"
    );

    // ── Database connections ────────────────────────────────────────────────

    let mut mongo_conn = MongoConnection::new(config.mongo.clone());
    let db = mongo_conn
        .connect()
        .await
        .expect("Failed to connect to MongoDB");

    // SeaORM connection pool for PostgreSQL
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

    // ── Message broker connection ───────────────────────────────────────────

    let mut rmq_conn = RabbitMqConnection::new(config.rabbitmq.clone());
    let channel = rmq_conn
        .connect()
        .await
        .expect("Failed to connect to RabbitMQ");

    // ── Repository layer (trait objects for DI) ────────────────────────────

    let user_repo: Arc<dyn server_monitoring::repository::user_repo::UserRepository> =
        Arc::new(MongoUserRepository::new(&db));

    let client_repo: Arc<dyn server_monitoring::repository::client_repo::ClientRepository> =
        Arc::new(MongoClientRepository::new(&db));

    let api_key_repo: Arc<dyn server_monitoring::repository::api_key_repo::ApiKeyRepository> =
        Arc::new(MongoApiKeyRepository::new(&db));

    let metrics_repo: Arc<dyn server_monitoring::repository::metrics_repo::MetricsRepository> =
        Arc::new(SeaOrmMetricsRepository::new(sea_db.clone()));

    // ── Service layer ───────────────────────────────────────────────────────

    let auth_service = AuthService::new(user_repo.clone(), config.clone());

    let client_service = ClientService::new(client_repo, api_key_repo, user_repo);

    let analytics_service = AnalyticsService::new(metrics_repo);

    // Event producer with circuit breaker + retry — all parameters from config
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

    let ingest_service = IngestService::new(event_producer);

    // ── Dependency injection container ──────────────────────────────────────

    let app_state = web::Data::new(AppState::new(
        auth_service,
        client_service,
        analytics_service,
        ingest_service,
        sea_db,
        config.clone(),
        circuit_breaker,
    ));

    let port = config.port;
    let config_clone = config.clone();

    tracing::info!("Server starting on port {}", port);

    // ── HTTP server ─────────────────────────────────────────────────────────

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
            // 404 fallback
            .default_service(web::route().to(|| async {
                HttpResponse::NotFound().json(ResponseFormatter::not_found("Endpoint not found"))
            }))
    })
    .bind(format!("0.0.0.0:{}", port))?
    // Allow 30 s for in-flight requests to complete on shutdown
    .shutdown_timeout(30)
    .run();

    // Obtain a handle for graceful shutdown signaling
    let server_handle = server.handle();

    // Spawn a background task that listens for CTRL+C and stops the server
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            tracing::info!("Received shutdown signal, stopping server gracefully...");
            server_handle.stop(true).await;
        }
    });

    server.await
}
