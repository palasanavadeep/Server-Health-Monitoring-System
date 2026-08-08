use tracing_appender::rolling;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

/// Initialize the tracing/logging subsystem.
///
/// - **Production**: JSON format, info level
/// - **Development**: pretty format, debug level
/// - File outputs: `logs/error.log` (error only), `logs/combined.log` (all)
/// - Console output always enabled
pub fn init_telemetry(environment: &str) {
    let is_production = environment == "production";

    let level = if is_production { "info" } else { "debug" };
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));

    // File appenders
    let error_file = rolling::never("logs", "error.log");
    let combined_file = rolling::never("logs", "combined.log");

    // Ensure logs directory exists
    std::fs::create_dir_all("logs").ok();

    if is_production {
        let error_layer = fmt::layer()
            .json()
            .with_writer(error_file)
            .with_filter(tracing_subscriber::filter::LevelFilter::ERROR);

        let combined_layer = fmt::layer().json().with_writer(combined_file);

        let console_layer = fmt::layer().json().with_writer(std::io::stdout);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(error_layer)
            .with(combined_layer)
            .with(console_layer)
            .init();
    } else {
        let error_layer = fmt::layer()
            .json()
            .with_writer(error_file)
            .with_filter(tracing_subscriber::filter::LevelFilter::ERROR);

        let combined_layer = fmt::layer().json().with_writer(combined_file);

        let console_layer = fmt::layer().pretty().with_writer(std::io::stdout);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(error_layer)
            .with(combined_layer)
            .with(console_layer)
            .init();
    }

    tracing::info!(service = "server-monitoring", "Logger initialized");
}
