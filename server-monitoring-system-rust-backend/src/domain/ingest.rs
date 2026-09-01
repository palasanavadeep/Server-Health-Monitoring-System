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

/// Internal event published to the metrics queue after MongoDB persistence.
///
/// ## Invariants
///
/// - `event_id`: globally unique, immutable per logical event. Assigned once by
///   `IngestService` using `Uuid::new_v4()`. Receiving the same `event_id` twice
///   means RabbitMQ redelivery — not a new event. The metrics worker uses this
///   for idempotent deduplication.
///
/// - `timestamp`: the wall-clock time when the original API hit occurred,
///   **not** when the consumer processes it. Used for time-bucket aggregation.
///   Must never be regenerated on retry — doing so would put events into the
///   wrong time bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricsEvent {
    pub event_id: String,
    pub client_id: String,
    pub service_name: String,
    pub endpoint: String,
    pub method: String,
    pub status_code: u16,
    pub latency_ms: f64,
    /// Time of the original API hit — determines the time-bucket for aggregation.
    pub timestamp: DateTime<Utc>,
}
