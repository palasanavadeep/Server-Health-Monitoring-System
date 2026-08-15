//! Typed DTOs for the analytics / metrics pipeline.
//!
//! These replace `serde_json::Value` at the repository → service → handler boundary.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Overall request statistics for a time range.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverallStats {
    pub total_hits: i64,
    pub error_hits: i64,
    pub avg_latency: f64,
    pub unique_services: i64,
    pub unique_endpoints: i64,
}

/// Per-endpoint aggregated statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndpointStat {
    pub service_name: String,
    pub endpoint: String,
    pub method: String,
    pub total_hits: i64,
    pub avg_latency: f64,
    pub error_hits: i64,
}

/// Single time-series bucket row.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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

/// Pagination metadata returned alongside list responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    pub page: i64,
    pub limit: i64,
    pub total_count: i64,
    pub total_pages: i64,
}

/// Paginated API metrics response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedApiMetrics {
    pub items: Vec<ApiMetricsEntry>,
    pub pagination: Pagination,
}

/// Dashboard snapshot — all panels in one response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardData {
    pub stats: Option<OverallStatsResponse>,
    pub top_endpoints: Option<Vec<EndpointStatResponse>>,
    /// Intentional typo preserved from original Node.js API contract.
    #[serde(rename = "recentActitivy")]
    pub recent_activity: Option<Vec<TimeSeriesEntryResponse>>,
}

/// HTTP response shape for overall stats (adds derived fields).
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

/// HTTP response shape for endpoint stats (adds error_rate).
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

/// HTTP response shape for time-series entry.
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

/// HTTP response shape for per-API metrics (adds derived fields).
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

/// Paginated API metrics HTTP response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedApiMetricsResponse {
    pub items: Vec<ApiMetricsEntryResponse>,
    pub pagination: Pagination,
}

/// Time-range bounds, included in stats response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}
