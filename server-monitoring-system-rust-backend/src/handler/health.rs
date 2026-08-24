use actix_web::{web, HttpResponse};
use sea_orm::DatabaseConnection;

use crate::app_state::AppState;

/// GET /health — liveness and readiness check.
///
/// Reports system status including circuit breaker state and DB connectivity.
pub async fn health(state: web::Data<AppState>) -> HttpResponse {
    let (cb_state, cb_failures, cb_successes) = state.circuit_breaker.get_stats();

    // Quick DB connectivity check
    let db_status = check_db_connectivity(&state.db).await;

    let overall_status =
        if db_status && cb_state == crate::resilience::circuit_breaker::CircuitState::Closed {
            "healthy"
        } else {
            "degraded"
        };

    HttpResponse::Ok().json(serde_json::json!({
        "status": overall_status,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "uptime": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        "components": {
            "database": {
                "status": if db_status { "up" } else { "down" }
            },
            "circuit_breaker": {
                "state": format!("{:?}", cb_state),
                "failures": cb_failures,
                "successes": cb_successes
            }
        }
    }))
}

/// GET / — service info endpoint.
pub async fn root() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "service": "Server Monitoring System",
        "version": "1.0.0",
        "endpoints": {
            "health": "/health",
            "auth": "/api/auth",
            "admin": "/api/admin",
            "ingest": "/api/hit",
            "analytics": "/api/analytics"
        }
    }))
}

/// Configure health and root routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/health", web::get().to(health))
        .route("/", web::get().to(root));
}

/// Quick PostgreSQL connectivity check via SeaORM.
async fn check_db_connectivity(db: &DatabaseConnection) -> bool {
    use sea_orm::ConnectionTrait;
    db.execute_unprepared("SELECT 1").await.is_ok()
}
