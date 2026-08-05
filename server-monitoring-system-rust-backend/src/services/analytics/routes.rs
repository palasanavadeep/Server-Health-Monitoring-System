use actix_web::web;

use crate::config::app_config::AppConfig;
use crate::middleware::authenticate::Authenticate;
use super::handlers;

/// Configure analytics routes - mirrors Node.js analyticsRoutes.js.
/// All routes require JWT authentication.
pub fn configure(cfg: &mut web::ServiceConfig, config: &AppConfig) {
    cfg.service(
        web::scope("/api/analytics")
            .wrap(Authenticate::new(config))
            .route("/stats", web::get().to(handlers::get_stats))
            .route("/dashboard", web::get().to(handlers::get_dashboard))
            .route("/apis", web::get().to(handlers::get_apis_metrics)),
    );
}
