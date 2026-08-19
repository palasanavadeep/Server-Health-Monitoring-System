//! Internal metrics domain types — used between repository and service layers.
//!
//! These are the raw aggregated data structures that repositories return.
//! They are **not** exposed directly in HTTP responses.
//! Response-facing shapes live in `dto/response/analytics.rs`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Raw overall statistics row from the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverallStats {
    pub total_hits: i64,
    pub error_hits: i64,
    pub avg_latency: f64,
    pub unique_services: i64,
    pub unique_endpoints: i64,
}

/// Raw per-endpoint aggregated statistics row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointStat {
    pub service_name: String,
    pub endpoint: String,
    pub method: String,
    pub total_hits: i64,
    pub avg_latency: f64,
    pub error_hits: i64,
}

/// Single time-series bucket row from the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesEntry {
    pub service_name: String,
    pub endpoint: String,
    pub method: String,
    pub total_hits: i64,
    pub error_hits: i64,
    pub avg_latency: f64,
    pub min_latency: f64,
    pub max_latency: f64,
    pub time_bucket: DateTime<Utc>,
}

/// Full per-endpoint metrics row (for the APIs table).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMetricsEntry {
    pub service_name: String,
    pub endpoint: String,
    pub method: String,
    pub total_hits: i64,
    pub error_hits: i64,
    pub avg_latency: f64,
    pub min_latency: f64,
    pub max_latency: f64,
}
