//! Typed DTOs for the ingest pipeline.
//!
//! These replace `serde_json::Value` at the handler → service → messaging boundary,
//! giving compile-time field guarantees on the hot path.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Incoming API hit event from the SDK.
///
/// Deserialized directly from the HTTP request body.
/// Fields `client_id`, `api_key_id`, `ip`, and `user_agent` are
/// injected by the ingest handler after API-key validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestHitRequest {
    pub service_name: String,
    pub endpoint: String,
    pub method: String,
    pub status_code: u16,
    pub latency_ms: f64,

    /// Injected by handler after API-key auth — not supplied by the caller.
    #[serde(skip_deserializing)]
    pub client_id: String,

    /// Injected by handler after API-key auth — not supplied by the caller.
    #[serde(skip_deserializing, default)]
    pub api_key_id: Option<String>,

    /// Injected by handler from request IP — not supplied by the caller.
    #[serde(skip_deserializing, default)]
    pub ip: Option<String>,

    /// Injected by handler from User-Agent header — not supplied by the caller.
    #[serde(skip_deserializing, default)]
    pub user_agent: Option<String>,
}

/// Result returned by `IngestService::ingest_api_hit`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestResult {
    pub event_id: String,
    pub status: IngestStatus,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Whether the event was accepted into the queue or rejected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestStatus {
    Queued,
    Rejected,
}

/// The envelope published onto RabbitMQ.
///
/// Typed so the consumer can deserialize directly without going through `Value`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HitEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: HitEventData,
    pub published_at: DateTime<Utc>,
    pub attempt: u32,
}

/// Payload inside the `HitEvent` envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HitEventData {
    pub event_id: String,
    pub timestamp: DateTime<Utc>,
    pub service_name: String,
    pub endpoint: String,
    pub method: String,
    pub status_code: u16,
    pub latency_ms: f64,
    pub client_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_id: Option<String>,
    #[serde(default = "default_ip")]
    pub ip: String,
    #[serde(default)]
    pub user_agent: String,
}

fn default_ip() -> String {
    "unknown".to_string()
}
