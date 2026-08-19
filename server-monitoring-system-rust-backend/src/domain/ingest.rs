//! Ingest messaging envelope — RabbitMQ event types.
//!
//! Only the **messaging envelope** lives here.
//! - HTTP request body → `dto/request/ingest.rs` (`IngestHitRequest`)
//! - HTTP response → `dto/response/ingest.rs` (`IngestResponse`, `IngestStatus`)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Envelope published onto RabbitMQ.
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

/// Payload inside the `HitEvent` envelope — written by producer, read by consumer.
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
