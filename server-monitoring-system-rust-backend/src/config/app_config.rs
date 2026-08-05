use std::env;

/// Application configuration loaded from environment variables.
/// Mirrors the Node.js config/index.js exactly.
#[derive(Debug, Clone)]
pub struct AppConfig {
    // Server
    pub node_env: String,
    pub port: u16,

    // MongoDB
    pub mongo: MongoConfig,

    // PostgreSQL
    pub postgres: PostgresConfig,

    // RabbitMQ
    pub rabbitmq: RabbitMqConfig,

    // JWT
    pub jwt: JwtConfig,

    // Rate Limit
    pub rate_limit: RateLimitConfig,

    // Cookie
    pub cookie: CookieConfig,

    // Valid API Keys (kept for backward compatibility, not used in code)
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

impl AppConfig {
    /// Load configuration from environment variables with defaults matching Node.js.
    pub fn from_env() -> Self {
        let node_env = env::var("NODE_ENV").unwrap_or_else(|_| "development".to_string());
        let is_production = node_env == "production";

        AppConfig {
            node_env: node_env.clone(),
            port: env::var("PORT")
                .unwrap_or_else(|_| "5000".to_string())
                .parse()
                .unwrap_or(5000),

            mongo: MongoConfig {
                uri: env::var("MONGO_URI")
                    .unwrap_or_else(|_| "mongodb://localhost:27017/server_monitoring".to_string()),
                db_name: env::var("MONGO_DB_NAME")
                    .unwrap_or_else(|_| "server_monitoring".to_string()),
            },

            postgres: PostgresConfig {
                host: env::var("PG_HOST").unwrap_or_else(|_| "localhost".to_string()),
                port: env::var("PG_PORT")
                    .unwrap_or_else(|_| "5432".to_string())
                    .parse()
                    .unwrap_or(5432),
                database: env::var("PG_DATABASE")
                    .unwrap_or_else(|_| "server_monitoring".to_string()),
                user: env::var("PG_USER").unwrap_or_else(|_| "postgres".to_string()),
                password: env::var("PG_PASSWORD").unwrap_or_else(|_| "postgres".to_string()),
            },

            rabbitmq: RabbitMqConfig {
                url: env::var("RABBITMQ_URL")
                    .unwrap_or_else(|_| "amqp://localhost:5672".to_string()),
                queue: env::var("RABBITMQ_QUEUE")
                    .unwrap_or_else(|_| "server_hits".to_string()),
                publisher_confirms: env::var("RABBITMQ_PUBLISHER_CONFIRMS")
                    .unwrap_or_else(|_| "false".to_string())
                    == "true",
                retry_attempts: env::var("RABBITMQ_RETRY_ATTEMPTS")
                    .unwrap_or_else(|_| "3".to_string())
                    .parse()
                    .unwrap_or(3),
                retry_delay: env::var("RABBITMQ_RETRY_DELAY")
                    .unwrap_or_else(|_| "1000".to_string())
                    .parse()
                    .unwrap_or(1000),
            },

            jwt: JwtConfig {
                secret: env::var("JWT_SECRET")
                    .unwrap_or_else(|_| "jwt_secret_key_is_ not a secret now".to_string()),
                expires_in: env::var("JWT_EXPIRES_IN")
                    .unwrap_or_else(|_| "24h".to_string()),
            },

            rate_limit: RateLimitConfig {
                window_ms: env::var("RATE_LIMIT_WINDOW_MS")
                    .unwrap_or_else(|_| "900000".to_string())
                    .parse()
                    .unwrap_or(900_000),
                max_requests: env::var("RATE_LIMIT_MAX_REQUESTS")
                    .unwrap_or_else(|_| "1000".to_string())
                    .parse()
                    .unwrap_or(1000),
            },

            cookie: CookieConfig {
                http_only: true,
                secure: is_production,
                same_site_lax: true,
                expires_in_ms: 24 * 60 * 60 * 1000, // 24 hours in ms
            },

            valid_api_keys: env::var("VALID_API_KEYS")
                .unwrap_or_default()
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|s| s.trim().to_string())
                .collect(),
        }
    }

    pub fn is_production(&self) -> bool {
        self.node_env == "production"
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
