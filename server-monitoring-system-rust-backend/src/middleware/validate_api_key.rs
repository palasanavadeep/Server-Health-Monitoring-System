use actix_web::{web, HttpRequest, HttpResponse};

use crate::app_state::AppState;
use crate::domain::api_key::ApiKey;
use crate::domain::client::Client;
use crate::util::ip::IpUtils;
use crate::util::response::ResponseFormatter;

/// Validated API key data attached to request extensions.
#[derive(Debug, Clone)]
pub struct ValidatedApiKeyData {
    pub client: Client,
    pub api_key: ApiKey,
}

/// Validate API key - mirrors Node.js validateApiKey.js middleware.
/// Implemented as an extractor/handler wrapper rather than Transform middleware
/// since it needs async database access through AppState.
pub async fn validate_api_key(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> Result<ValidatedApiKeyData, HttpResponse> {
    // Read x-api-key header
    let api_key_value = req
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let api_key_value = match api_key_value {
        Some(k) if !k.is_empty() => k,
        _ => {
            return Err(HttpResponse::Unauthorized().json(ResponseFormatter::error(
                "API key is required",
                401,
                None,
            )));
        }
    };

    // Look up the API key via ClientService
    let result = state
        .client_service
        .get_client_by_api_key(&api_key_value)
        .await;

    let (client, api_key) = match result {
        Ok(Some((client, api_key))) => (client, api_key),
        Ok(None) => {
            return Err(HttpResponse::Unauthorized().json(ResponseFormatter::error(
                "Invalid API key",
                401,
                None,
            )));
        }
        Err(e) => {
            tracing::error!("Error validating API key: {}", e);
            return Err(
                HttpResponse::InternalServerError().json(ResponseFormatter::error(
                    "Internal server error",
                    500,
                    None,
                )),
            );
        }
    };

    // Check if client is active
    if !client.is_active {
        return Err(HttpResponse::Forbidden().json(ResponseFormatter::error(
            "Client account is deactivated",
            403,
            None,
        )));
    }

    // Check IP restrictions
    let client_ip = req
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("unknown")
        .to_string();

    if !IpUtils::is_ip_allowed(&client_ip, &api_key.security.allowed_i_ps) {
        tracing::warn!(
            "IP {} not in allowed list for API key {}",
            client_ip,
            api_key.key_id
        );
        return Err(HttpResponse::Forbidden().json(ResponseFormatter::error(
            "IP address not allowed",
            403,
            None,
        )));
    }

    // Check Origin restrictions
    if let Some(origin) = req.headers().get("origin").and_then(|v| v.to_str().ok()) {
        let allowed_origins = &api_key.security.allowed_origins;
        if !allowed_origins.iter().any(|o| o == "*" || o == origin) {
            tracing::warn!(
                "Origin {} not in allowed list for API key {}",
                origin,
                api_key.key_id
            );
            return Err(HttpResponse::Forbidden().json(ResponseFormatter::error(
                "Origin not allowed",
                403,
                None,
            )));
        }
    }

    // Check canIngest permission
    if !api_key.permissions.can_ingest {
        return Err(HttpResponse::Forbidden().json(ResponseFormatter::error(
            "API key does not have ingest permission",
            403,
            None,
        )));
    }

    Ok(ValidatedApiKeyData { client, api_key })
}
