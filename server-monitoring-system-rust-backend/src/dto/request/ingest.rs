//! Ingest request DTO — the body received from SDK callers.

use serde::{Deserialize, Serialize};
use validator::Validate;

/// POST /api/hit — ingest a single API hit event.
///
/// `client_id`, `api_key_id`, `ip`, and `user_agent` are **not** supplied by
/// the caller — they are injected by the handler after API-key validation.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct IngestHitRequest {
    #[validate(length(min = 1, message = "serviceName is required"))]
    pub service_name: String,

    #[validate(length(min = 1, message = "endpoint is required"))]
    pub endpoint: String,

    #[validate(length(min = 1, message = "method is required"))]
    pub method: String,

    pub status_code: u16,
    pub latency_ms: f64,

    /// Injected by handler after API-key auth — not supplied by caller.
    #[serde(skip_deserializing, default)]
    pub client_id: String,

    /// Injected by handler after API-key auth — not supplied by caller.
    #[serde(skip_deserializing, default)]
    pub api_key_id: Option<String>,

    /// Injected from request remote IP — not supplied by caller.
    #[serde(skip_deserializing, default)]
    pub ip: Option<String>,

    /// Injected from User-Agent header — not supplied by caller.
    #[serde(skip_deserializing, default)]
    pub user_agent: Option<String>,
}
