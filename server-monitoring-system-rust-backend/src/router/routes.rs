use actix_web::web;

use crate::config::settings::AppConfig;
use crate::handler::{analytics, auth, client, health, ingest};
use crate::middleware::authenticate::Authenticate;

/// Mount all application routes.
///
/// Route structure mirrors the original Node.js express router exactly so
/// existing API consumers are not affected.
pub fn configure(cfg: &mut web::ServiceConfig, config: &AppConfig) {
    // ── Health & Root ───────────────────────────────────────────────────────
    cfg.route("/health", web::get().to(health::health))
        .route("/", web::get().to(health::root));

    // ── Auth routes ─────────────────────────────────────────────────────────
    cfg.service(
        web::scope("/api/auth")
            // Public
            .route("/onboard-super-admin", web::post().to(auth::onboard_super_admin))
            .route("/login", web::post().to(auth::login))
            .route("/logout", web::get().to(auth::logout))
            // JWT-protected
            .service(
                web::scope("")
                    .wrap(Authenticate::new(config))
                    .route("/register", web::post().to(auth::register))
                    .route("/profile", web::get().to(auth::get_profile))
                    .route("/profile", web::put().to(auth::update_profile))
                    .route("/users/{userId}/deactivate", web::patch().to(auth::deactivate_user)),
            ),
    );

    // ── Admin / Client management routes (all require JWT) ──────────────────
    cfg.service(
        web::scope("/api")
            .wrap(Authenticate::new(config))
            .route("/admin/clients/onboard", web::post().to(client::create_client))
            .route("/admin/clients/{clientId}/users", web::post().to(client::create_client_user))
            .route("/admin/clients/{clientId}/api/keys", web::post().to(client::create_api_key))
            .route("/admin/clients/{clientId}/api/keys", web::get().to(client::get_client_api_keys))
            .route("/admin/clients/{client_id}/api/keys/{key_id}", web::put().to(client::update_api_key))
            .route("/admin/clients/{client_id}/api/keys/{key_id}", web::delete().to(client::delete_api_key))
            .route("/admin/clients/{client_id}/api/keys/{key_id}/deactivate", web::patch().to(client::deactivate_api_key))
            .route("/admin/clients/{client_id}/api/keys/{key_id}/activate", web::patch().to(client::activate_api_key))
            .route("/admin/clients/{client_id}/api/keys/{key_id}/rotate", web::post().to(client::rotate_api_key))
            .route("/admin/clients/{client_id}/api/keys/{key_id}", web::get().to(client::get_api_key)),
    );

    // ── Analytics routes (JWT-protected) ────────────────────────────────────
    cfg.service(
        web::scope("/api/analytics")
            .wrap(Authenticate::new(config))
            .route("/stats", web::get().to(analytics::get_stats))
            .route("/dashboard", web::get().to(analytics::get_dashboard))
            .route("/apis", web::get().to(analytics::get_apis_metrics)),
    );

    // ── Ingest routes (API-key-protected, no JWT) ────────────────────────────
    cfg.service(
        web::scope("/api/hit")
            .route("/", web::post().to(ingest::ingest_hit)),
    );
}
