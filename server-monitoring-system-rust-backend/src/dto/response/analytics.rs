//! Analytics response DTOs — shapes returned to API consumers.
//!
//! These are derived from internal domain types
//! (`OverallStats`, `EndpointStat`, etc.) by the analytics service.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── Overall stats ─────────────────────────────────────────────────────────────

/// GET /api/analytics/stats response shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverallStatsResponse {
    pub total_hits: i64,
    pub error_hits: i64,
    pub success_hits: i64,
    pub error_rate: f64,
    pub avg_latency: f64,
    pub unique_services: i64,
    pub unique_endpoints: i64,
    pub time_range: TimeRange,
}

/// Inclusive time range returned alongside stats.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

// ── Endpoint stats ────────────────────────────────────────────────────────────

/// Single row in the top-endpoints list.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndpointStatResponse {
    pub service_name: String,
    pub endpoint: String,
    pub method: String,
    pub total_hits: i64,
    pub avg_latency: f64,
    pub error_hits: i64,
    pub error_rate: f64,
}

// ── Time series ───────────────────────────────────────────────────────────────

/// Single time-bucket row in the activity feed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeSeriesEntryResponse {
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

// ── Per-API metrics ───────────────────────────────────────────────────────────

/// Single row in the APIs metrics table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiMetricsEntryResponse {
    pub service_name: String,
    pub endpoint: String,
    pub method: String,
    pub total_hits: i64,
    pub error_hits: i64,
    pub success_hits: i64,
    pub error_rate: f64,
    pub avg_latency: f64,
    pub min_latency: f64,
    pub max_latency: f64,
}

// ── Dashboard ─────────────────────────────────────────────────────────────────

/// GET /api/analytics/dashboard — full dashboard snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardResponse {
    pub stats: Option<OverallStatsResponse>,
    pub top_endpoints: Option<Vec<EndpointStatResponse>>,
    /// Intentional field name preserved from Node.js API contract.
    #[serde(rename = "recentActitivy")]
    pub recent_activity: Option<Vec<TimeSeriesEntryResponse>>,
}

// ── Pagination ────────────────────────────────────────────────────────────────

/// Pagination metadata included in list responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationMeta {
    pub page: i64,
    pub limit: i64,
    pub total_count: i64,
    pub total_pages: i64,
}
