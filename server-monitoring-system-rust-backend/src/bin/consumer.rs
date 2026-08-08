use std::sync::Arc;

use server_monitoring::config::database::{MongoConnection, create_pg_pool};
use server_monitoring::config::messaging::RabbitMqConnection;
use server_monitoring::config::settings::AppConfig;
use server_monitoring::config::telemetry;
use server_monitoring::messaging::consumer::EventConsumer;
use server_monitoring::repository::api_hit_repo::MongoApiHitRepository;
use server_monitoring::repository::metrics_repo::PgMetricsRepository;
use server_monitoring::resilience::circuit_breaker::CircuitBreaker;
use server_monitoring::resilience::retry::RetryStrategy;
use server_monitoring::service::processor::ProcessorService;

#[tokio::main]
async fn main() {
    // Load environment variables
    dotenvy::dotenv().ok();

    let config = AppConfig::from_env();

    // Initialize structured logging
    telemetry::init_telemetry(&config.environment);

    tracing::info!("Starting Server Monitoring Consumer (Rust)");

    // Startup retry loop (mirrors Node.js startConsumerWithRetry)
    let startup_retry = RetryStrategy::new(5, 5000, 30_000, 0.3);
    let mut attempt: u32 = 0;

    loop {
        tracing::info!("Starting consumer (attempt {})", attempt + 1);

        match start_consumer(&config).await {
            Ok(consumer) => {
                tracing::info!("Consumer started successfully");

                // Block until CTRL+C
                tokio::signal::ctrl_c()
                    .await
                    .expect("Failed to listen for ctrl+c");

                tracing::info!("Received shutdown signal, stopping gracefully...");
                consumer.stop();

                // Give time for in-flight messages to complete
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
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
                    tracing::error!("Max retries reached, exiting...");
                    std::process::exit(1);
                }

                startup_retry.wait(attempt - 1).await;
            }
        }
    }
}

async fn start_consumer(
    config: &AppConfig,
) -> Result<EventConsumer, Box<dyn std::error::Error + Send + Sync>> {
    // Connect to databases with retry
    let max_retries = 5u32;
    let mut retries = 0u32;

    let (db, pg_pool) = loop {
        tracing::info!("Connecting to databases...");

        let mongo_result = async {
            let mut conn = MongoConnection::new(config.mongo.clone());
            conn.connect()
                .await
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("{}", e).into()
                })
        };

        let pg_result = async {
            create_pg_pool(&config.postgres_connection_string())
                .await
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
        };

        match tokio::try_join!(mongo_result, pg_result) {
            Ok((db, pool)) => {
                tracing::info!("Database connections established");
                break (db, pool);
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

                tokio::time::sleep(tokio::time::Duration::from_secs(5 * retries as u64)).await;
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
        Arc::new(PgMetricsRepository::new(pg_pool));

    // Service layer
    let processor_service = Arc::new(ProcessorService::new(api_hit_repo, metrics_repo));

    // Resilience primitives for the consumer
    let circuit_breaker = Arc::new(CircuitBreaker::new(5, 30_000, 3));
    let retry_strategy = Arc::new(RetryStrategy::new(
        config.rabbitmq.retry_attempts,
        config.rabbitmq.retry_delay,
        30_000,
        0.3,
    ));

    let consumer = EventConsumer::new(
        processor_service,
        channel,
        config.rabbitmq.queue.clone(),
        retry_strategy,
        circuit_breaker,
    );

    consumer.start().await?;

    Ok(consumer)
}
