use actix_web::{web, HttpRequest, HttpResponse};

use crate::app_state::AppState;
use crate::middleware::validate_api_key;
use crate::util::response::ResponseFormatter;

/// POST /api/hit/
pub async fn ingest_hit(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    // Validate API key via shared middleware function
    let validated = match validate_api_key::validate_api_key(req.clone(), state.clone()).await {
        Ok(v) => v,
        Err(response) => return response,
    };

    tracing::info!(
        client_id = ?validated.client.id,
        client_name = %validated.client.name,
        "Ingest: Client data received"
    );

    let mut hit_data = body.into_inner();

    // Inject client/key metadata (auth context enrichment)
    if let serde_json::Value::Object(ref mut map) = hit_data {
        map.insert(
            "clientId".to_string(),
            serde_json::json!(validated.client.id.map(|id| id.to_hex())),
        );
        map.insert(
            "apiKeyId".to_string(),
            serde_json::json!(validated.api_key.id.map(|id| id.to_hex())),
        );
        let ip = req
            .connection_info()
            .realip_remote_addr()
            .unwrap_or("unknown")
            .to_string();
        map.insert("ip".to_string(), serde_json::json!(ip));
        let user_agent = req
            .headers()
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        map.insert("userAgent".to_string(), serde_json::json!(user_agent));
    }

    tracing::info!(
        client_id = ?validated.client.id,
        endpoint = hit_data.get("endpoint").and_then(|v| v.as_str()).unwrap_or(""),
        method = hit_data.get("method").and_then(|v| v.as_str()).unwrap_or(""),
        "Ingest: Hit data prepared"
    );

    match state.ingest_service.ingest_api_hit(hit_data).await {
        Ok(result) => {
            if result.get("status").and_then(|v| v.as_str()) == Some("rejected") {
                HttpResponse::ServiceUnavailable().json(ResponseFormatter::error(
                    "Service temporarily unavailable",
                    503,
                    Some(serde_json::json!({
                        "eventId": result.get("eventId"),
                        "reason": result.get("reason"),
                        "retryAfter": "30 seconds"
                    })),
                ))
            } else {
                HttpResponse::Accepted().json(ResponseFormatter::success(
                    result,
                    "API hit queued for processing",
                    202,
                ))
            }
        }
        Err(e) => e.to_response(),
    }
}
