use actix_web::{web, HttpRequest, HttpResponse};

use crate::app_state::AppState;
use crate::cache::api_key_cache::CachedApiKeyEntry;
use crate::util::response::ResponseFormatter;
use crate::util::ip::IpUtils;

/// Validated API key data passed to ingest handlers.
///
/// Populated from the `ApiKeyCache` (served from MongoDB on cache miss).
/// Contains only the fields needed downstream — no raw key value.
#[derive(Debug, Clone)]
pub struct ValidatedApiKeyData {
    pub client_id:   String,
    pub key_id:      String,
    pub client_active: bool,
}

/// Validate the `X-Api-Key` header against the in-process `ApiKeyCache`.
///
/// ## Cache behaviour
/// - **Hit:** validation runs in memory — zero DB calls.
/// - **Miss:** a single MongoDB lookup populates the cache entry; concurrent
///   misses for the same key collapse into one DB call via moka single-flight.
///
/// ## Checks performed (in order)
/// 1. Header presence
/// 2. Cache / DB lookup
/// 3. Key active flag
/// 4. Key expiry
/// 5. Client active flag
/// 6. IP allow-list
/// 7. Origin allow-list (only when `Origin` header is present)
/// 8. `can_ingest` permission
pub async fn validate_api_key(
    req:   HttpRequest,
    state: web::Data<AppState>,
) -> Result<ValidatedApiKeyData, HttpResponse> {
    // ── 1. Extract raw key from header ─────────────────────────────────────────
    let raw_key = req
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let raw_key = match raw_key {
        Some(k) if !k.is_empty() => k,
        _ => {
            return Err(HttpResponse::Unauthorized().json(ResponseFormatter::error(
                "API key is required",
                401,
                None,
            )));
        }
    };

    // ── 2. Lookup via cache (single-flight on miss) ─────────────────────────────
    let entry: std::sync::Arc<CachedApiKeyEntry> = match state
        .api_key_cache
        .get_or_load(&raw_key, || {
            let state_inner = state.clone();
            let key_inner   = raw_key.clone();
            async move {
                let result = state_inner
                    .client_service
                    .get_client_by_api_key(&key_inner)
                    .await?;

                match result {
                    None => Err(crate::error::app_error::AppError::not_found("API key not found")),
                    Some((client, api_key)) => Ok(CachedApiKeyEntry {
                        client_id:      client.id.clone().unwrap_or_default(),
                        key_id:         api_key.key_id.clone(),
                        permissions:    api_key.permissions.clone(),
                        security:       api_key.security.clone(),
                        is_active:      api_key.is_active,
                        expires_at:     api_key.expires_at,
                        client_active:  client.is_active,
                    }),
                }
            }
        })
        .await
    {
        Ok(e) => e,
        Err(_) => {
            return Err(HttpResponse::Unauthorized().json(ResponseFormatter::error(
                "Invalid API key",
                401,
                None,
            )));
        }
    };

    // ── 3. Key active ───────────────────────────────────────────────────────────
    if !entry.is_active {
        return Err(HttpResponse::Unauthorized().json(ResponseFormatter::error(
            "API key is inactive",
            401,
            None,
        )));
    }

    // ── 4. Key expiry ───────────────────────────────────────────────────────────
    if let Some(expires_at) = entry.expires_at {
        if chrono::Utc::now() > expires_at {
            tracing::warn!(key_id = %entry.key_id, "Expired API key attempted access");
            return Err(HttpResponse::Unauthorized().json(ResponseFormatter::error(
                "API key expired",
                401,
                None,
            )));
        }
    }

    // ── 5. Client active ────────────────────────────────────────────────────────
    if !entry.client_active {
        return Err(HttpResponse::Forbidden().json(ResponseFormatter::error(
            "Client account is deactivated",
            403,
            None,
        )));
    }

    // ── 6. IP allow-list ────────────────────────────────────────────────────────
    let client_ip = req
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("unknown")
        .to_string();

    if !IpUtils::is_ip_allowed(&client_ip, &entry.security.allowed_i_ps) {
        tracing::warn!(
            key_id    = %entry.key_id,
            client_ip = %client_ip,
            "IP not in allowed list for API key"
        );
        return Err(HttpResponse::Forbidden().json(ResponseFormatter::error(
            "IP address not allowed",
            403,
            None,
        )));
    }

    // ── 7. Origin allow-list (optional header) ──────────────────────────────────
    if let Some(origin) = req.headers().get("origin").and_then(|v| v.to_str().ok()) {
        let allowed = &entry.security.allowed_origins;
        if !allowed.iter().any(|o| o == "*" || o == origin) {
            tracing::warn!(
                key_id = %entry.key_id,
                origin = %origin,
                "Origin not in allowed list for API key"
            );
            return Err(HttpResponse::Forbidden().json(ResponseFormatter::error(
                "Origin not allowed",
                403,
                None,
            )));
        }
    }

    // ── 8. Ingest permission ────────────────────────────────────────────────────
    if !entry.permissions.can_ingest {
        return Err(HttpResponse::Forbidden().json(ResponseFormatter::error(
            "API key does not have ingest permission",
            403,
            None,
        )));
    }

    tracing::debug!(
        key_id    = %entry.key_id,
        client_id = %entry.client_id,
        "API key validated (served from cache)"
    );

    Ok(ValidatedApiKeyData {
        client_id:     entry.client_id.clone(),
        key_id:        entry.key_id.clone(),
        client_active: entry.client_active,
    })
}
