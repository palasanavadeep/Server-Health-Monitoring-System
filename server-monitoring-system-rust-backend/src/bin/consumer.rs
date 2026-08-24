use std::sync::Arc;

use server_monitoring::config::database::{create_sea_orm_db, MongoConnection};
use server_monitoring::config::messaging::RabbitMqConnection;
use server_monitoring::config::settings::AppConfig;
use server_monitoring::config::telemetry;
use server_monitoring::messaging::consumer::EventConsumer;
use server_monitoring::repository::api_hit_repo::MongoApiHitRepository;
use server_monitoring::repository::metrics_repo::SeaOrmMetricsRepository;
use server_monitoring::resilience::circuit_breaker::CircuitBreaker;
use server_monitoring::resilience::retry::RetryStrategy;
use server_monitoring::service::processor::ProcessorService;

#[tokio::main]
async fn main() {
    // Load environment variables from .env file (no-op if file missing)
    dotenvy::dotenv().ok();

    // Load and validate typed configuration — panics if required env vars are missing
    let config = AppConfig::from_env();

    // Initialize structured logging
    telemetry::init_telemetry(&config.environment);

    tracing::info!("Starting Server Monitoring Consumer (Rust)");

    // Startup retry strategy — parameters driven by ConsumerConfig
    let startup_retry = RetryStrategy::new(
        config.consumer.startup_max_retries,
        config.consumer.startup_base_delay_ms,
        config.resilience.retry_max_delay_ms,
        config.resilience.retry_jitter_factor,
    );
    let mut attempt: u32 = 0;

    loop {
        tracing::info!(attempt = attempt + 1, "Starting consumer");

        match start_consumer(&config).await {
            Ok((consumer, handle)) => {
                tracing::info!("Consumer started successfully");

                // Block until CTRL+C
                tokio::signal::ctrl_c()
                    .await
                    .expect("Failed to listen for ctrl+c");

                tracing::info!("Received shutdown signal, stopping gracefully...");
                consumer.stop();

                // Wait for the consume loop to finish draining, with a timeout
                let drain_timeout =
                    tokio::time::Duration::from_secs(config.consumer.graceful_shutdown_secs);
                if tokio::time::timeout(drain_timeout, handle).await.is_err() {
                    tracing::warn!("Consumer did not drain within timeout, forcing exit");
                }
                break;
            }
            Err(e) => {
                attempt += 1;
                tracing::error!(
                    attempt = attempt,
                    error = %e,
                    "Consumer start attempt failed"
                );

                if !startup_retry.should_retry(attempt) {
                    tracing::error!(
                        max_retries = config.consumer.startup_max_retries,
                        "Max retries reached, exiting"
                    );
                    std::process::exit(1);
                }

                startup_retry.wait(attempt - 1).await;
            }
        }
    }
}

async fn start_consumer(
    config: &AppConfig,
) -> Result<(EventConsumer, tokio::task::JoinHandle<()>), Box<dyn std::error::Error + Send + Sync>>
{
    let max_retries = config.consumer.db_connect_max_retries;
    let mut retries = 0u32;

    let (db, sea_db) = loop {
        tracing::info!(attempt = retries + 1, "Connecting to databases");

        let mongo_result = async {
            let mut conn = MongoConnection::new(config.mongo.clone());
            conn.connect()
                .await
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("MongoDB: {}", e).into()
                })
        };

        let pg_result = async {
            create_sea_orm_db(&config.postgres_connection_string())
                .await
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("PostgreSQL (SeaORM): {}", e).into()
                })
        };

        match tokio::try_join!(mongo_result, pg_result) {
            Ok((mongo_db, sea_db)) => {
                tracing::info!("Database connections established");
                break (mongo_db, sea_db);
            }
            Err(e) => {
                retries += 1;
                tracing::error!(
                    attempt = retries,
                    error = %e,
                    "Database connection attempt failed"
                );

                if retries >= max_retries {
                    return Err(format!(
                        "Failed to connect to databases after {} attempts",
                        max_retries
                    )
                    .into());
                }

                let delay_secs = (config.consumer.startup_base_delay_ms / 1_000) * retries as u64;
                tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs)).await;
            }
        }
    };

    // Connect to RabbitMQ
    let mut rmq_conn = RabbitMqConnection::new(config.rabbitmq.clone());
    let channel = rmq_conn.connect().await?;

    // Repository layer
    let api_hit_repo: Arc<dyn server_monitoring::repository::api_hit_repo::ApiHitRepository> =
        Arc::new(MongoApiHitRepository::new(&db));

    let metrics_repo: Arc<dyn server_monitoring::repository::metrics_repo::MetricsRepository> =
        Arc::new(SeaOrmMetricsRepository::new(sea_db));

    // Service layer
    let processor_service = Arc::new(ProcessorService::new(api_hit_repo, metrics_repo));

    // Resilience primitives — all parameters from config
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

    let consumer = EventConsumer::new(
        processor_service,
        channel,
        config.rabbitmq.queue.clone(),
        retry_strategy,
        circuit_breaker,
        config.consumer.idempotency_cache_size,
    );

    let handle = consumer.start().await?;

    Ok((consumer, handle))
}
