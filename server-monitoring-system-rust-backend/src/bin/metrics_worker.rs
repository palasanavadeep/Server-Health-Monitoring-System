use std::sync::Arc;

use server_monitoring::config::database::create_sea_orm_db;
use server_monitoring::config::messaging::RabbitMqConnection;
use server_monitoring::config::settings::AppConfig;
use server_monitoring::config::telemetry;
use server_monitoring::messaging::metrics_consumer::MetricsConsumer;
use server_monitoring::repository::metrics_repo::SeaOrmMetricsRepository;
use server_monitoring::repository::tenant_config_repo::SeaOrmTenantConfigRepository;
use server_monitoring::resilience::circuit_breaker::CircuitBreaker;
use server_monitoring::resilience::retry::RetryStrategy;
use server_monitoring::service::metrics_processor::MetricsProcessorService;


#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let config = AppConfig::from_env();

    telemetry::init_telemetry(&config.environment);

    tracing::info!("Starting Server Monitoring Metrics Worker (Rust)");

    let startup_retry = RetryStrategy::new(
        config.consumer.startup_max_retries,
        config.consumer.startup_base_delay_ms,
        config.resilience.retry_max_delay_ms,
        config.resilience.retry_jitter_factor,
    );
    let mut attempt: u32 = 0;

    loop {
        tracing::info!(attempt = attempt + 1, "Starting metrics worker");

        match start_metrics_worker(&config).await {
            Ok((consumer, handle)) => {
                tracing::info!("Metrics worker started successfully");

                tokio::signal::ctrl_c()
                    .await
                    .expect("Failed to listen for ctrl+c");

                tracing::info!("Shutdown signal received — stopping metrics worker");
                consumer.stop();

                let drain_timeout =
                    tokio::time::Duration::from_secs(config.consumer.graceful_shutdown_secs);
                if tokio::time::timeout(drain_timeout, handle).await.is_err() {
                    tracing::warn!("Metrics worker did not drain within timeout");
                }
                break;
            }
            Err(e) => {
                attempt += 1;
                tracing::error!(attempt = attempt, error = %e, "Metrics worker start failed");

                if !startup_retry.should_retry(attempt) {
                    tracing::error!(
                        max_retries = config.consumer.startup_max_retries,
                        "Max retries reached — exiting metrics worker"
                    );
                    std::process::exit(1);
                }

                startup_retry.wait(attempt - 1).await;
            }
        }
    }
}

/// Connect to PostgreSQL + RabbitMQ and start the metrics consumer.
///
/// This worker does NOT connect to MongoDB — that is the persistence consumer's
/// responsibility. The narrow dependency surface makes this worker independently
/// scalable and deployable.
async fn start_metrics_worker(
    config: &AppConfig,
) -> Result<
    (MetricsConsumer, tokio::task::JoinHandle<()>),
    Box<dyn std::error::Error + Send + Sync>,
> {
    let max_retries = config.consumer.db_connect_max_retries;
    let mut retries = 0u32;

    // Connect to PostgreSQL with retry.
    let pg_db = loop {
        tracing::info!(attempt = retries + 1, "Connecting to PostgreSQL");

        match create_sea_orm_db(&config.postgres_connection_string()).await {
            Ok(db) => {
                tracing::info!("PostgreSQL connection established");
                break db;
            }
            Err(e) => {
                retries += 1;
                tracing::error!(attempt = retries, error = %e, "PostgreSQL connection failed");

                if retries >= max_retries {
                    return Err(
                        format!("Failed to connect to PostgreSQL after {max_retries} attempts")
                            .into(),
                    );
                }

                let delay =
                    tokio::time::Duration::from_millis(config.consumer.startup_base_delay_ms);
                tokio::time::sleep(delay).await;
            }
        }
    };

    // Connect to RabbitMQ (also declares all queues — startup-critical).
    let mut rmq_conn = RabbitMqConnection::new(config.rabbitmq.clone());
    let channel = rmq_conn.connect().await?;

    // Repository and service.
    let metrics_repo          = Arc::new(SeaOrmMetricsRepository::new(pg_db.clone()));
    let tenant_config_repo    = Arc::new(SeaOrmTenantConfigRepository::new(pg_db));
    let metrics_processor     = Arc::new(MetricsProcessorService::new(
        metrics_repo,
        tenant_config_repo,
        config.ingest.dedup_retention_days,
    ));


    // Resilience primitives.
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

    // Subscribe to the metrics queue (server_hits.metrics).
    let metrics_queue = config.rabbitmq.metrics_queue();

    let consumer = MetricsConsumer::new(
        metrics_processor,
        channel,
        metrics_queue,
        retry_strategy,
        circuit_breaker,
        config.consumer.idempotency_cache_size,
    );

    let handle = consumer.start().await?;

    Ok((consumer, handle))
}
