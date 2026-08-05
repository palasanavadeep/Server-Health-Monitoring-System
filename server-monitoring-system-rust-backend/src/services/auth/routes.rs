use actix_web::web;

use crate::config::app_config::AppConfig;
use crate::middleware::authenticate::Authenticate;
use super::handlers;

/// Configure auth routes - mirrors Node.js authRoutes.js.
pub fn configure(cfg: &mut web::ServiceConfig, config: &AppConfig) {
    cfg.service(
        web::scope("/api/auth")
            // Public routes (no auth)
            .route(
                "/onboard-super-admin",
                web::post().to(handlers::onboard_super_admin),
            )
            .route("/login", web::post().to(handlers::login))
            .route("/logout", web::get().to(handlers::logout))
            // Protected routes (require JWT)
            .service(
                web::scope("")
                    .wrap(Authenticate::new(config))
                    .route("/register", web::post().to(handlers::register))
                    .route("/profile", web::get().to(handlers::get_profile))
                    .route("/profile", web::put().to(handlers::update_profile))
                    .route(
                        "/users/{userId}/deactivate",
                        web::patch().to(handlers::deactivate_user),
                    ),
            ),
    );
}
