//! Analytics response DTOs — the shapes returned to API consumers.
//!
//! Derived from internal domain types by the service layer.
//! No business logic lives here; these are pure serialization containers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::metrics::{ApdexScore, LatencyPercentiles, ServiceHealthStatus, StatusDistribution};

// ── Shared ─────────────────────────────────────────────────────────────────────

/// Inclusive time range returned alongside query results.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end:   DateTime<Utc>,
}

// ── Overall stats ─────────────────────────────────────────────────────────────

/// `GET /api/analytics/stats` — aggregated statistics for a time range.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverallStatsResponse {
    pub total_hits:       i64,
    pub error_hits:       i64,
    pub success_hits:     i64,
    pub error_rate:       f64,
    pub avg_latency:      f64,
    pub unique_services:  i64,
    pub unique_endpoints: i64,
    pub time_range:       TimeRange,
}

// ── Top-endpoints list ────────────────────────────────────────────────────────

/// Single row in the top-endpoints list.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndpointStatResponse {
    pub service_name: String,
    pub endpoint:     String,
    pub method:       String,
    pub total_hits:   i64,
    pub avg_latency:  f64,
    pub error_hits:   i64,
    pub error_rate:   f64,
}

// ── Time series ───────────────────────────────────────────────────────────────

/// Single time-bucket row in the activity feed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeSeriesEntryResponse {
    pub service_name: String,
    pub endpoint:     String,
    pub method:       String,
    pub total_hits:   i64,
    pub error_hits:   i64,
    pub avg_latency:  f64,
    pub min_latency:  f64,
    pub max_latency:  f64,
    pub time_bucket:  DateTime<Utc>,
}

// ── Per-API metrics ───────────────────────────────────────────────────────────

/// `GET /api/analytics/apis` — single row in the paginated endpoint table.
///
/// Includes full computed performance metrics derived from the stored histogram
/// and counter columns. All fields are stable; new fields are additive.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiMetricsResponse {
    pub service_name:        String,
    pub endpoint:            String,
    pub method:              String,
    pub total_hits:          i64,
    pub error_hits:          i64,
    pub success_hits:        i64,
    pub error_rate:          f64,
    pub avg_latency:         f64,
    pub min_latency:         f64,
    pub max_latency:         f64,
    /// Latency percentile estimates (ms): p50, p75, p90, p95, p99.
    pub percentiles:         LatencyPercentiles,
    /// HTTP response status class distribution.
    pub status_distribution: StatusDistribution,
    /// Apdex score and component counts.
    pub apdex:               ApdexScore,
    /// Requests per minute over the queried window (0.0 for all-time queries).
    pub throughput_rpm:      f64,
}

// ── Endpoint metrics (percentiles endpoint) ───────────────────────────────────

/// `GET /api/analytics/percentiles` — full computed metrics for a single endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndpointMetricsResponse {
    pub percentiles:         LatencyPercentiles,
    pub status_distribution: StatusDistribution,
    pub apdex:               ApdexScore,
    /// Requests per minute over the queried time window.
    pub throughput_rpm:      f64,
    pub time_range:          TimeRange,
}

// ── Dashboard ─────────────────────────────────────────────────────────────────

/// `GET /api/analytics/dashboard` — composite snapshot for the main dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardResponse {
    pub stats:         Option<OverallStatsResponse>,
    pub top_endpoints: Option<Vec<EndpointStatResponse>>,
    /// NOTE: Field name intentionally preserved from the original Node.js API
    /// contract. Fixing the typo would be a breaking change for all clients.
    #[serde(rename = "recentActitivy")]
    pub recent_activity: Option<Vec<TimeSeriesEntryResponse>>,
}

// ── Pagination ────────────────────────────────────────────────────────────────

/// Pagination metadata included in list responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationMeta {
    pub page:        i64,
    pub limit:       i64,
    pub total_count: i64,
    pub total_pages: i64,
}

// ── Service-level analytics ───────────────────────────────────────────────────

/// One row in the `GET /api/analytics/services` (overview) response.
///
/// Health status is derived at read time from Apdex + error rate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceHealthResponse {
    pub service_name:        String,
    pub total_hits:          i64,
    pub error_hits:          i64,
    pub success_hits:        i64,
    pub error_rate:          f64,
    pub avg_latency:         f64,
    /// p99 derived from the summed service histogram.
    pub p99_latency:         f64,
    /// Apdex score (0.0 – 1.0).
    pub apdex:               f64,
    /// Requests per minute over the queried window.
    pub throughput_rpm:      f64,
    /// Number of distinct (endpoint, method) pairs in this service.
    pub endpoint_count:      i64,
    /// Derived health classification.
    pub status:              ServiceHealthStatus,
    pub status_distribution: StatusDistribution,
}

/// Summary metrics within `GET /api/analytics/services?serviceName=…`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceSummaryResponse {
    pub total_hits:          i64,
    pub error_hits:          i64,
    pub success_hits:        i64,
    pub error_rate:          f64,
    pub avg_latency:         f64,
    pub min_latency:         f64,
    pub max_latency:         f64,
    pub throughput_rpm:      f64,
    pub percentiles:         LatencyPercentiles,
    pub apdex:               ApdexScore,
    pub status_distribution: StatusDistribution,
    pub status:              ServiceHealthStatus,
}

/// Endpoint entry in the service detail endpoint list.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceEndpointStatResponse {
    pub endpoint:    String,
    pub method:      String,
    pub total_hits:  i64,
    pub error_hits:  i64,
    pub error_rate:  f64,
    pub avg_latency: f64,
    pub p99_latency: f64,
    pub apdex_score: f64,
}

/// `GET /api/analytics/services?serviceName=…` — full metrics for one service.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceMetricsResponse {
    pub service_name: String,
    pub summary:      ServiceSummaryResponse,
    pub endpoints:    Vec<ServiceEndpointStatResponse>,
    pub time_range:   TimeRange,
}
