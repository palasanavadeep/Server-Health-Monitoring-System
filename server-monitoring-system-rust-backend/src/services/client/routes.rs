use actix_web::web;

use crate::config::app_config::AppConfig;
use crate::middleware::authenticate::Authenticate;
use super::handlers;

/// Configure client routes - mirrors Node.js clientRoutes.js.
/// All routes require JWT authentication (applied at scope level).
pub fn configure(cfg: &mut web::ServiceConfig, config: &AppConfig) {
    cfg.service(
        web::scope("/api")
            .wrap(Authenticate::new(config))
            // Client onboarding
            .route("/admin/clients/onboard", web::post().to(handlers::create_client))
            // Client users
            .route("/admin/clients/{clientId}/users", web::post().to(handlers::create_client_user))
            // API keys - CRUD
            .route("/admin/clients/{clientId}/api/keys", web::post().to(handlers::create_api_key))
            .route("/admin/clients/{clientId}/api/keys", web::get().to(handlers::get_client_api_keys))
            // API key operations with keyId
            .route("/admin/clients/{client_id}/api/keys/{key_id}", web::put().to(handlers::update_api_key))
            .route("/admin/clients/{client_id}/api/keys/{key_id}", web::delete().to(handlers::delete_api_key))
            .route("/admin/clients/{client_id}/api/keys/{key_id}/deactivate", web::patch().to(handlers::deactivate_api_key))
            .route("/admin/clients/{client_id}/api/keys/{key_id}/activate", web::patch().to(handlers::activate_api_key))
            .route("/admin/clients/{client_id}/api/keys/{key_id}/rotate", web::post().to(handlers::rotate_api_key))
            .route("/admin/clients/{client_id}/api/keys/{key_id}", web::get().to(handlers::get_api_key)),
    );
}
