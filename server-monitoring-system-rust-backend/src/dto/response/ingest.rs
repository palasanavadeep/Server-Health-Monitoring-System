//! Ingest response DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Outcome of a single ingest request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestResponse {
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
