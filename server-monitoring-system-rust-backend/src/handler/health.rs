use actix_web::{web, HttpResponse};

/// GET /health — liveness check.
pub async fn health() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "uptime": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
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
