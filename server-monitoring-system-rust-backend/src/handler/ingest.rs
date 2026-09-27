use actix_web::{web, HttpRequest, HttpResponse};

use crate::app_state::AppState;
use crate::dto::request::ingest::IngestHitRequest;
use crate::dto::response::ingest::IngestStatus;
use crate::middleware::validate_api_key;
use crate::util::response::ResponseFormatter;

// ── Single-event ingest ───────────────────────────────────────────────────────

/// `POST /api/hit`
///
/// Validates the API key via the in-process cache (zero DB calls on hit),
/// then delegates to `IngestService::ingest_hit`.
pub async fn ingest_hit(
    state: web::Data<AppState>,
    req:   HttpRequest,
    body:  web::Json<IngestHitRequest>,
) -> HttpResponse {
    let validated = match validate_api_key::validate_api_key(req.clone(), state.clone()).await {
        Ok(v)  => v,
        Err(r) => return r,
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

    let mut hit    = body.into_inner();
    hit.client_id  = validated.client_id.clone();
    hit.api_key_id = Some(validated.key_id.clone());
    hit.ip         = Some(ip);
    hit.user_agent = Some(user_agent);

    let client_id = hit.client_id.clone();

    // Load tenant config for quota check (served from cache, ~0 latency on hit)
    let tenant_config = match state.tenant_config_service.get_config(&client_id).await {
        Ok(c)  => c,
        Err(e) => return e.to_response(),
    };

    match state.ingest_service.ingest_hit(hit, &tenant_config).await {
        Ok(result) => {
            if result.status == IngestStatus::Rejected {
                if result.reason.as_deref().map_or(false, |r| r.starts_with("daily_quota_exceeded")) {
                    return HttpResponse::TooManyRequests().json(ResponseFormatter::error(
                        "Daily ingest quota exceeded",
                        429,
                        result.reason.as_ref().map(|r| serde_json::json!({ "detail": r })),
                    ));
                }
                HttpResponse::ServiceUnavailable().json(ResponseFormatter::error(
                    "Service temporarily unavailable",
                    503,
                    Some(serde_json::json!({
                        "eventId":    result.event_id,
                        "reason":     result.reason,
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

// ── Batch ingest ──────────────────────────────────────────────────────────────

/// `POST /api/hits`
///
/// Accepts up to `BATCH_INGEST_MAX_EVENTS` events in a single request.
/// Returns 207 Multi-Status when some events were accepted and others rejected.
/// Quota is checked cumulatively for the whole batch upfront.
pub async fn ingest_batch(
    state: web::Data<AppState>,
    req:   HttpRequest,
    body:  web::Json<BatchIngestRequestBody>,
) -> HttpResponse {
    if body.events.is_empty() {
        return HttpResponse::BadRequest().json(ResponseFormatter::error(
            "Batch must contain at least one event",
            400,
            None,
        ));
    }

    let validated = match validate_api_key::validate_api_key(req.clone(), state.clone()).await {
        Ok(v)  => v,
        Err(r) => return r,
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

    let client_id  = validated.client_id.clone();
    let api_key_id = Some(validated.key_id.clone());

    // Inject auth context into every event
    let events: Vec<IngestHitRequest> = body.into_inner().events.into_iter().map(|mut e| {
        e.client_id  = client_id.clone();
        e.api_key_id = api_key_id.clone();
        e.ip         = Some(ip.clone());
        e.user_agent = Some(user_agent.clone());
        e
    }).collect();

    let tenant_config = match state.tenant_config_service.get_config(&client_id).await {
        Ok(c)  => c,
        Err(e) => return e.to_response(),
    };

    let max_batch = state.config.ingest.batch_max_events;

    match state
        .ingest_service
        .ingest_batch(events, &tenant_config, max_batch)
        .await
    {
        Ok(result) => {
            if result.accepted == 0
                && result.rejected.first().map_or(false, |e| {
                    e.reason.starts_with("daily_quota_exceeded")
                })
            {
                return HttpResponse::TooManyRequests().json(ResponseFormatter::error(
                    "Daily ingest quota exceeded",
                    429,
                    Some(serde_json::json!({ "detail": result.rejected[0].reason })),
                ));
            }

            let status_code = if result.is_fully_accepted() { 202 } else { 207 };
            let message = if result.is_fully_accepted() {
                "All events queued for processing"
            } else {
                "Batch partially accepted"
            };

            match status_code {
                202 => HttpResponse::Accepted()
                    .json(ResponseFormatter::success(result, message, 202)),
                _ => HttpResponse::MultiStatus()
                    .json(ResponseFormatter::success(result, message, 207)),
            }
        }
        Err(e) => e.to_response(),
    }
}

// ── Request body ──────────────────────────────────────────────────────────────

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchIngestRequestBody {
    pub events: Vec<IngestHitRequest>,
}
