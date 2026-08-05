use actix_web::web;

use crate::config::app_config::AppConfig;
use crate::middleware::rate_limiter::RateLimiter;
use super::handlers;

/// Configure ingest routes - mirrors Node.js ingestRoutes.js.
/// POST /api/hit/ with validateApiKey (in handler) + rateLimit middleware.
pub fn configure(cfg: &mut web::ServiceConfig, config: &AppConfig) {
    cfg.service(
        web::scope("/api/hit")
            .wrap(RateLimiter::new(
                config.rate_limit.window_ms,
                config.rate_limit.max_requests,
            ))
            .route("/", web::post().to(handlers::ingest_hit)),
    );
}
