//! Ingest response DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── Single-event response ─────────────────────────────────────────────────────

/// Outcome of a single ingest request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestResponse {
    pub event_id:  String,
    pub status:    IngestStatus,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason:    Option<String>,
}

impl IngestResponse {
    /// Convenience constructor for a quota-exceeded response.
    pub fn quota_exceeded(count: i64, limit: i64) -> Self {
        Self {
            event_id:  String::new(),
            status:    IngestStatus::Rejected,
            timestamp: Utc::now(),
            reason:    Some(format!("daily_quota_exceeded:{count}/{limit}")),
        }
    }
}

/// Whether the event was accepted into the queue or rejected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestStatus {
    Queued,
    Rejected,
}

// ── Batch ingest response ─────────────────────────────────────────────────────

/// Outcome of a batch ingest request.
///
/// Follows partial-success (207 Multi-Status) semantics when some events were
/// accepted and others rejected due to validation failures.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchIngestResponse {
    pub accepted: usize,
    pub rejected: Vec<BatchEventError>,
    pub total:    usize,
}

impl BatchIngestResponse {
    /// All events were accepted.
    pub fn all_accepted(total: usize) -> Self {
        Self { accepted: total, rejected: vec![], total }
    }

    /// Batch rejected due to quota — no events processed.
    pub fn quota_exceeded(count: i64, limit: i64) -> Self {
        Self {
            accepted: 0,
            total:    0,
            rejected: vec![BatchEventError {
                index:  0,
                reason: format!("daily_quota_exceeded:{count}/{limit}"),
            }],
        }
    }

    /// Returns `true` when every event was accepted.
    pub fn is_fully_accepted(&self) -> bool {
        self.rejected.is_empty()
    }
}

/// A single event that failed to be ingested within a batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchEventError {
    /// Zero-based index into the original `events` array.
    pub index:  usize,
    /// Human-readable reason for the failure.
    pub reason: String,
}
