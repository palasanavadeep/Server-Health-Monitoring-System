use actix_web::{web, App, HttpServer, HttpResponse};
use std::sync::Arc;
use std::time::Instant;

use server_monitoring::config::app_config::AppConfig;
use server_monitoring::config::logger;
use server_monitoring::config::mongodb::MongoConnection;
use server_monitoring::config::postgres;
use server_monitoring::config::rabbitmq::RabbitMqConnection;
use server_monitoring::app_state::AppState;
use server_monitoring::middleware::request_logger::RequestLogger;
use server_monitoring::utils::response_formatter::ResponseFormatter;

// Services
use server_monitoring::services::auth::repository::UserRepository;
use server_monitoring::services::auth::service::AuthService;
use server_monitoring::services::client::client_repository::ClientRepository;
use server_monitoring::services::client::api_key_repository::ApiKeyRepository;
use server_monitoring::services::client::service::ClientService;
use server_monitoring::services::ingest::service::IngestService;
use server_monitoring::services::analytics::service::AnalyticsService;
use server_monitoring::services::processor::metrics_repository::MetricsRepository;

// Events
use server_monitoring::events::producer::circuit_breaker::CircuitBreaker;
use server_monitoring::events::producer::retry_strategy::RetryStrategy;
use server_monitoring::events::producer::event_producer::EventProducer;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load .env
    dotenvy::dotenv().ok();

    // Load config
    let config = AppConfig::from_env();

    // Initialize logger
    logger::init_logger(&config.node_env);

    tracing::info!(
        port = config.port,
        env = %config.node_env,
        "Starting Server Monitoring System (Rust)"
    );

    let _start_time = Instant::now();

    // Connect to MongoDB
    let mut mongo_conn = MongoConnection::new(config.mongo.clone());
    let db = mongo_conn.connect().await.expect("Failed to connect to MongoDB");

    // Connect to PostgreSQL
    let pg_pool = postgres::create_pool(&config.postgres_connection_string())
        .await
        .expect("Failed to connect to PostgreSQL");

    // Connect to RabbitMQ
    let mut rmq_conn = RabbitMqConnection::new(config.rabbitmq.clone());
    let channel = rmq_conn.connect().await.expect("Failed to connect to RabbitMQ");

    // Build repositories
    let user_repo = UserRepository::new(&db);
    let client_repo = ClientRepository::new(&db);
    let api_key_repo = ApiKeyRepository::new(&db);
    let _metrics_repo = MetricsRepository::new(pg_pool.clone());

    // Build services
    let auth_service = Arc::new(AuthService::new(user_repo, config.clone()));

    let user_repo_for_client = UserRepository::new(&db);
    let client_service = Arc::new(ClientService::new(
        client_repo,
        api_key_repo,
        user_repo_for_client,
    ));

    // Event producer with circuit breaker and retry
    let circuit_breaker = Arc::new(CircuitBreaker::new(2, 30_000, 3));
    let retry_strategy = Arc::new(RetryStrategy::new(
        config.rabbitmq.retry_attempts,
        config.rabbitmq.retry_delay,
        30_000,
        0.3,
    ));
    let event_producer = EventProducer::new(
        channel,
        config.rabbitmq.clone(),
        circuit_breaker,
        retry_strategy,
    );

    let ingest_service = Arc::new(IngestService::new(event_producer));

    let analytics_repo = MetricsRepository::new(pg_pool.clone());
    let analytics_service = Arc::new(AnalyticsService::new(analytics_repo));

    let app_state = web::Data::new(AppState {
        config: config.clone(),
        auth_service,
        client_service,
        ingest_service,
        analytics_service,
    });

    let port = config.port;
    let config_clone = config.clone();

    tracing::info!("Server starting on port {}", port);

    HttpServer::new(move || {
        let cors = actix_cors::Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .supports_credentials();

        App::new()
            .app_data(app_state.clone())
            .wrap(cors)
            .wrap(RequestLogger)
            // Health endpoint
            .route("/health", web::get().to(|| async {
                HttpResponse::Ok().json(serde_json::json!({
                    "status": "healthy",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "uptime": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                }))
            }))
            // Root endpoint
            .route("/", web::get().to(|| async {
                HttpResponse::Ok().json(serde_json::json!({
                    "service": "Server Monitoring System",
                    "version": "1.0.0",
                    "endpoints": {
                        "health": "/health",
                        "auth": "/api/auth",
                        "admin": "/api/admin",
                        "ingest": "/api/hit",
                        "analytics": "/api/analytics"
                    }
                }))
            }))
            // Service routes
            .configure(|cfg| {
                server_monitoring::services::auth::routes::configure(cfg, &config_clone);
            })
            .configure(|cfg| {
                server_monitoring::services::client::routes::configure(cfg, &config_clone);
            })
            .configure(|cfg| {
                server_monitoring::services::ingest::routes::configure(cfg, &config_clone);
            })
            .configure(|cfg| {
                server_monitoring::services::analytics::routes::configure(cfg, &config_clone);
            })
            // 404 handler
            .default_service(web::route().to(|| async {
                HttpResponse::NotFound().json(ResponseFormatter::error(
                    "Endpoint not found",
                    404,
                    None,
                ))
            }))
    })
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await
}
