use std::env;

// ── Defaults ───────────────────────────────────────────────────────────────────
// All magic numbers live here. Every hardcoded literal in the codebase
// should ultimately trace back to one of these constants.

const DEFAULT_PORT: u16 = 5000;
const DEFAULT_MONGO_URI: &str = "mongodb://localhost:27017/server_monitoring";
const DEFAULT_MONGO_DB: &str = "server_monitoring";
const DEFAULT_PG_HOST: &str = "localhost";
const DEFAULT_PG_PORT: u16 = 5432;
const DEFAULT_PG_DATABASE: &str = "server_monitoring";
const DEFAULT_PG_USER: &str = "postgres";
const DEFAULT_RABBITMQ_URL: &str = "amqp://guest:guest@localhost:5672/";
const DEFAULT_RABBITMQ_QUEUE: &str = "server_hits";
const DEFAULT_RABBITMQ_RETRY_ATTEMPTS: u32 = 3;
const DEFAULT_RABBITMQ_RETRY_DELAY_MS: u64 = 1_000;
const DEFAULT_JWT_EXPIRES_IN: &str = "24h";
const DEFAULT_RATE_LIMIT_WINDOW_MS: u64 = 60_000;
const DEFAULT_RATE_LIMIT_MAX_REQUESTS: u64 = 100;
const DEFAULT_COOKIE_EXPIRES_IN_MS: u64 = 24 * 60 * 60 * 1_000; // 24 h

// Resilience defaults
const DEFAULT_CB_FAILURE_THRESHOLD: u32 = 5;
const DEFAULT_CB_COOLDOWN_MS: u64 = 30_000;
const DEFAULT_CB_HALF_OPEN_ATTEMPTS: u32 = 3;
const DEFAULT_RETRY_MAX_DELAY_MS: u64 = 30_000;
const DEFAULT_RETRY_JITTER_FACTOR: f64 = 0.3;

// Password policy defaults
const DEFAULT_PASSWORD_MIN_LENGTH: usize = 8;

// Consumer defaults
const DEFAULT_CONSUMER_STARTUP_MAX_RETRIES: u32 = 5;
const DEFAULT_CONSUMER_STARTUP_BASE_DELAY_MS: u64 = 5_000;
const DEFAULT_CONSUMER_DB_CONNECT_MAX_RETRIES: u32 = 5;
const DEFAULT_CONSUMER_GRACEFUL_SHUTDOWN_SECS: u64 = 2;
const DEFAULT_CONSUMER_IDEMPOTENCY_CACHE_SIZE: usize = 100_000;

// ── Config structs ─────────────────────────────────────────────────────────────

/// Application configuration loaded from environment variables.
///
/// All defaults are declared as typed constants above for easy discovery.
/// Call `AppConfig::from_env()` at startup and pass `config` everywhere
/// — never read `env::var` outside this module.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Application environment (reads `NODE_ENV` env var).
    pub environment: String,
    pub port: u16,

    pub mongo: MongoConfig,
    pub postgres: PostgresConfig,
    pub rabbitmq: RabbitMqConfig,
    pub jwt: JwtConfig,
    pub rate_limit: RateLimitConfig,
    pub cookie: CookieConfig,
    pub resilience: ResilienceConfig,
    pub consumer: ConsumerConfig,
    pub password_policy: PasswordPolicyConfig,

    /// Legacy API key list (kept for backward compatibility).
    pub valid_api_keys: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MongoConfig {
    pub uri: String,
    pub db_name: String,
}

#[derive(Debug, Clone)]
pub struct PostgresConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub password: String,
}

#[derive(Debug, Clone)]
pub struct RabbitMqConfig {
    pub url: String,
    pub queue: String,
    pub publisher_confirms: bool,
    pub retry_attempts: u32,
    pub retry_delay: u64,
}

#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub expires_in: String,
}

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub window_ms: u64,
    pub max_requests: u64,
}

#[derive(Debug, Clone)]
pub struct CookieConfig {
    pub http_only: bool,
    pub secure: bool,
    pub same_site_lax: bool,
    pub expires_in_ms: u64,
}

/// Resilience parameters — circuit breaker and retry knobs.
///
/// All values are readable from environment variables so they can be
/// tuned at runtime without recompiling.
#[derive(Debug, Clone)]
pub struct ResilienceConfig {
    /// Number of consecutive failures before the circuit opens.
    /// Env: `RESILIENCE_CB_FAILURE_THRESHOLD`
    pub cb_failure_threshold: u32,

    /// Time (ms) the circuit stays open before transitioning to half-open.
    /// Env: `RESILIENCE_CB_COOLDOWN_MS`
    pub cb_cooldown_ms: u64,

    /// Successful probe requests needed to close the circuit from half-open.
    /// Env: `RESILIENCE_CB_HALF_OPEN_ATTEMPTS`
    pub cb_half_open_attempts: u32,

    /// Upper bound on exponential backoff delay (ms).
    /// Env: `RESILIENCE_RETRY_MAX_DELAY_MS`
    pub retry_max_delay_ms: u64,

    /// Random jitter fraction applied to backoff delay (0.0 – 1.0).
    /// Env: `RESILIENCE_RETRY_JITTER_FACTOR`
    pub retry_jitter_factor: f64,
}

/// Password strength policy — loaded once at startup.
#[derive(Debug, Clone)]
pub struct PasswordPolicyConfig {
    pub min_length: usize,
    pub require_uppercase: bool,
    pub require_lowercase: bool,
    pub require_numbers: bool,
    pub require_symbols: bool,
}

/// Consumer-specific operational parameters.
///
/// Tuned separately from the API server's resilience config so the consumer
/// can be operated independently.
#[derive(Debug, Clone)]
pub struct ConsumerConfig {
    /// Max startup retry attempts before exiting.
    /// Env: `CONSUMER_STARTUP_MAX_RETRIES`
    pub startup_max_retries: u32,

    /// Base delay (ms) between startup retry attempts.
    /// Env: `CONSUMER_STARTUP_BASE_DELAY_MS`
    pub startup_base_delay_ms: u64,

    /// Max attempts to connect to databases on consumer startup.
    /// Env: `CONSUMER_DB_CONNECT_MAX_RETRIES`
    pub db_connect_max_retries: u32,

    /// Seconds to wait for in-flight messages to drain on graceful shutdown.
    /// Env: `CONSUMER_GRACEFUL_SHUTDOWN_SECS`
    pub graceful_shutdown_secs: u64,

    /// Maximum number of processed event IDs kept in the idempotency cache.
    /// Bounded to prevent unbounded memory growth.
    /// Env: `CONSUMER_IDEMPOTENCY_CACHE_SIZE`
    pub idempotency_cache_size: usize,
}

// ── Loader ─────────────────────────────────────────────────────────────────────

impl AppConfig {
    /// Load all configuration from environment variables.
    ///
    /// Call `dotenvy::dotenv().ok()` before this in `main`.
    /// Panics immediately on missing required secrets via `validate()`.
    pub fn from_env() -> Self {
        let environment = env::var("NODE_ENV").unwrap_or_else(|_| "development".to_string());
        let is_production = environment == "production";

        let config = AppConfig {
            environment: environment.clone(),
            port: parse_env("PORT", DEFAULT_PORT),

            mongo: MongoConfig {
                uri: env_or("MONGO_URI", DEFAULT_MONGO_URI),
                db_name: env_or("MONGO_DB_NAME", DEFAULT_MONGO_DB),
            },

            postgres: PostgresConfig {
                host: env_or("PG_HOST", DEFAULT_PG_HOST),
                port: parse_env("PG_PORT", DEFAULT_PG_PORT),
                database: env_or("PG_DATABASE", DEFAULT_PG_DATABASE),
                user: env_or("PG_USER", DEFAULT_PG_USER),
                // No default — will be caught by validate() if empty.
                password: env::var("PG_PASSWORD").unwrap_or_default(),
            },

            rabbitmq: RabbitMqConfig {
                url: env_or("RABBITMQ_URL", DEFAULT_RABBITMQ_URL),
                queue: env_or("RABBITMQ_QUEUE", DEFAULT_RABBITMQ_QUEUE),
                publisher_confirms: env_flag("RABBITMQ_PUBLISHER_CONFIRMS"),
                retry_attempts: parse_env(
                    "RABBITMQ_RETRY_ATTEMPTS",
                    DEFAULT_RABBITMQ_RETRY_ATTEMPTS,
                ),
                retry_delay: parse_env("RABBITMQ_RETRY_DELAY", DEFAULT_RABBITMQ_RETRY_DELAY_MS),
            },

            jwt: JwtConfig {
                // No default — will be caught by validate() if empty.
                secret: env::var("JWT_SECRET").unwrap_or_default(),
                expires_in: env_or("JWT_EXPIRES_IN", DEFAULT_JWT_EXPIRES_IN),
            },

            rate_limit: RateLimitConfig {
                window_ms: parse_env("RATE_LIMIT_WINDOW_MS", DEFAULT_RATE_LIMIT_WINDOW_MS),
                max_requests: parse_env("RATE_LIMIT_MAX_REQUESTS", DEFAULT_RATE_LIMIT_MAX_REQUESTS),
            },

            cookie: CookieConfig {
                http_only: true,
                secure: is_production,
                same_site_lax: true,
                expires_in_ms: parse_env("COOKIE_EXPIRES_IN_MS", DEFAULT_COOKIE_EXPIRES_IN_MS),
            },

            resilience: ResilienceConfig {
                cb_failure_threshold: parse_env(
                    "RESILIENCE_CB_FAILURE_THRESHOLD",
                    DEFAULT_CB_FAILURE_THRESHOLD,
                ),
                cb_cooldown_ms: parse_env("RESILIENCE_CB_COOLDOWN_MS", DEFAULT_CB_COOLDOWN_MS),
                cb_half_open_attempts: parse_env(
                    "RESILIENCE_CB_HALF_OPEN_ATTEMPTS",
                    DEFAULT_CB_HALF_OPEN_ATTEMPTS,
                ),
                retry_max_delay_ms: parse_env(
                    "RESILIENCE_RETRY_MAX_DELAY_MS",
                    DEFAULT_RETRY_MAX_DELAY_MS,
                ),
                retry_jitter_factor: parse_env_f64(
                    "RESILIENCE_RETRY_JITTER_FACTOR",
                    DEFAULT_RETRY_JITTER_FACTOR,
                ),
            },

            consumer: ConsumerConfig {
                startup_max_retries: parse_env(
                    "CONSUMER_STARTUP_MAX_RETRIES",
                    DEFAULT_CONSUMER_STARTUP_MAX_RETRIES,
                ),
                startup_base_delay_ms: parse_env(
                    "CONSUMER_STARTUP_BASE_DELAY_MS",
                    DEFAULT_CONSUMER_STARTUP_BASE_DELAY_MS,
                ),
                db_connect_max_retries: parse_env(
                    "CONSUMER_DB_CONNECT_MAX_RETRIES",
                    DEFAULT_CONSUMER_DB_CONNECT_MAX_RETRIES,
                ),
                graceful_shutdown_secs: parse_env(
                    "CONSUMER_GRACEFUL_SHUTDOWN_SECS",
                    DEFAULT_CONSUMER_GRACEFUL_SHUTDOWN_SECS,
                ),
                idempotency_cache_size: parse_env(
                    "CONSUMER_IDEMPOTENCY_CACHE_SIZE",
                    DEFAULT_CONSUMER_IDEMPOTENCY_CACHE_SIZE,
                ),
            },

            password_policy: PasswordPolicyConfig {
                min_length: parse_env("PASSWORD_MIN_LENGTH", DEFAULT_PASSWORD_MIN_LENGTH),
                require_uppercase: env_flag_or("PASSWORD_REQUIRE_UPPERCASE", true),
                require_lowercase: env_flag_or("PASSWORD_REQUIRE_LOWERCASE", true),
                require_numbers: env_flag_or("PASSWORD_REQUIRE_NUMBERS", true),
                require_symbols: env_flag_or("PASSWORD_REQUIRE_SYMBOLS", true),
            },

            valid_api_keys: env::var("VALID_API_KEYS")
                .unwrap_or_default()
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|s| s.trim().to_string())
                .collect(),
        };

        config.validate();
        config
    }

    /// Validate required configuration fields.
    ///
    /// Panics with a clear, actionable message if any required field is missing
    /// rather than allowing the app to start with broken config and fail later.
    fn validate(&self) {
        let mut missing: Vec<&str> = vec![];

        if self.postgres.password.is_empty() {
            missing.push("PG_PASSWORD");
        }
        if self.jwt.secret.is_empty() {
            missing.push("JWT_SECRET");
        }

        if !missing.is_empty() {
            panic!(
                "\n\n[CONFIG ERROR] Required environment variables are not set:\n\
                 {}\n\n\
                 Set them in your .env file or export them before starting.\n\
                 See .env.sample for all available options.\n",
                missing
                    .iter()
                    .map(|k| format!("  - {}", k))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
    }

    /// Check if running in production mode.
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }

    /// Build PostgreSQL connection string.
    pub fn postgres_connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.postgres.user,
            self.postgres.password,
            self.postgres.host,
            self.postgres.port,
            self.postgres.database
        )
    }
}

// ── Private helpers ────────────────────────────────────────────────────────────

/// Read an env var and parse it, falling back to the typed default.
fn parse_env<T: std::str::FromStr>(key: &str, default: T) -> T {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Read an env var and parse as f64, falling back to the typed default.
fn parse_env_f64(key: &str, default: f64) -> f64 {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Read a string env var, falling back to a `&str` default.
fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Read a boolean env var (truthy if value == `"true"`, case-insensitive).
fn env_flag(key: &str) -> bool {
    env::var(key)
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false)
}

/// Read a boolean env var with a non-false default.
fn env_flag_or(key: &str, default: bool) -> bool {
    match env::var(key) {
        Ok(v) => v.to_lowercase() == "true",
        Err(_) => default,
    }
}
