use actix_web::{web, HttpRequest, HttpResponse};
use serde_json::json;

use crate::app_state::AppState;
use crate::domain::ingest::{IngestHitRequest, IngestStatus};
use crate::middleware::validate_api_key;
use crate::util::response::ResponseFormatter;

/// POST /api/hit/
///
/// Accepts a typed `IngestHitRequest` body; injects auth context (client_id,
/// api_key_id, ip, user_agent) before forwarding to `IngestService`.
pub async fn ingest_hit(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<IngestHitRequest>,
) -> HttpResponse {
    // Validate API key and extract client context
    let validated = match validate_api_key::validate_api_key(req.clone(), state.clone()).await {
        Ok(v) => v,
        Err(response) => return response,
    };

    let ip = req
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("unknown")
        .to_string();

    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    // Inject auth context into the typed request
    let mut hit = body.into_inner();
    hit.client_id = validated
        .client
        .id
        .map(|id| id.to_hex())
        .unwrap_or_default();
    hit.api_key_id = validated.api_key.id.map(|id| id.to_hex());
    hit.ip = Some(ip);
    hit.user_agent = Some(user_agent);

    tracing::info!(
        client_id = %hit.client_id,
        endpoint = %hit.endpoint,
        method = %hit.method,
        "Ingest: hit data prepared"
    );

    match state.ingest_service.ingest_api_hit(hit).await {
        Ok(result) => {
            if result.status == IngestStatus::Rejected {
                HttpResponse::ServiceUnavailable().json(ResponseFormatter::error(
                    "Service temporarily unavailable",
                    503,
                    Some(json!({
                        "eventId": result.event_id,
                        "reason": result.reason,
                        "retryAfter": "30 seconds"
                    })),
                ))
            } else {
                HttpResponse::Accepted().json(ResponseFormatter::success(
                    json!(result),
                    "API hit queued for processing",
                    202,
                ))
            }
        }
        Err(e) => e.to_response(),
    }
}
