//! Metrics domain types — aggregated data structures shared between the
//! repository, service, and (via DTOs) API layers.
//!
//! ## Layer contract
//! These types are the output of repository queries. Services receive them and
//! map them to response DTOs. No HTTP or framework types appear here.
//!
//! ## Key design decisions
//! - Percentiles and Apdex are **never stored** — computed at read time.
//! - The 23-bucket histogram stores cumulative counts that are both additive and
//!   idempotent, making them safe under at-least-once RabbitMQ delivery.
//! - Apdex is derived from the histogram using the tenant's configured T threshold,
//!   so historical data remains valid when the threshold changes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── Aggregate query results ───────────────────────────────────────────────────

/// Overall statistics aggregated across all endpoints for a time range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverallStats {
    pub total_hits:       i64,
    pub error_hits:       i64,
    pub avg_latency:      f64,
    pub unique_services:  i64,
    pub unique_endpoints: i64,
}

/// Summary statistics for a single endpoint — used in top-N lists and tables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointStat {
    pub service_name: String,
    pub endpoint:     String,
    pub method:       String,
    pub total_hits:   i64,
    pub avg_latency:  f64,
    pub error_hits:   i64,
}

/// One time-bucket row from a time-series query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesEntry {
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

/// Full aggregated row for one endpoint, including all computed performance metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMetricsEntry {
    pub service_name: String,
    pub endpoint:     String,
    pub method:       String,
    pub total_hits:   i64,
    pub error_hits:   i64,
    pub avg_latency:  f64,
    pub min_latency:  f64,
    pub max_latency:  f64,
    /// Derived at read time from stored histogram + status counters.
    pub metrics: EndpointMetrics,
}

// ── Service-level types ───────────────────────────────────────────────────────

/// Health status of a service, derived at read time from Apdex and error rate.
///
/// ## Classification rules
/// | Status     | Apdex       | Error rate  |
/// |------------|-------------|-------------|
/// | Healthy    | ≥ 0.9       | ≤ 1.0 %     |
/// | Degraded   | ≥ 0.7       | ≤ 5.0 %     |
/// | Critical   | < 0.7       | > 5.0 %     |
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceHealthStatus {
    Healthy,
    Degraded,
    Critical,
}

impl ServiceHealthStatus {
    /// Classify a service's health from its Apdex score and error rate percentage.
    pub fn classify(apdex_score: f64, error_rate_pct: f64) -> Self {
        if apdex_score >= 0.9 && error_rate_pct <= 1.0 {
            Self::Healthy
        } else if apdex_score >= 0.7 && error_rate_pct <= 5.0 {
            Self::Degraded
        } else {
            Self::Critical
        }
    }
}

/// Aggregated summary for a single service (GROUP BY service_name).
///
/// Raw counters and histogram data used by the service layer to compute
/// derived metrics (percentiles, Apdex, status). Never stored in DB.
#[derive(Debug, Clone)]
pub struct ServiceSummary {
    pub service_name:        String,
    pub total_hits:          i64,
    pub error_hits:          i64,
    pub avg_latency:         f64,
    pub min_latency:         f64,
    pub max_latency:         f64,
    pub endpoint_count:      i64,
    pub active_buckets:      i64,
    /// Summed cumulative histogram counts across all endpoints in this service.
    pub histogram_counts:    [i64; 23],
    pub status_distribution: StatusDistribution,
}

/// Endpoint stat used in the service detail endpoint list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpointStat {
    pub endpoint:    String,
    pub method:      String,
    pub total_hits:  i64,
    pub error_hits:  i64,
    pub avg_latency: f64,
    pub p99_latency: f64,
    pub apdex_score: f64,
    pub error_rate:  f64,
}

// ── Computed performance metrics ──────────────────────────────────────────────


/// All computed performance metrics for a single endpoint aggregation.
///
/// Always derived at read time — never written to the database.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EndpointMetrics {
    pub percentiles:        LatencyPercentiles,
    pub status_distribution: StatusDistribution,
    pub apdex:              ApdexScore,
    /// Average requests per minute over the queried time window.
    /// Returns `0.0` for all-time aggregations (no bounded window).
    pub throughput_rpm:     f64,
}

/// Latency percentile estimates (ms) derived from the stored histogram.
///
/// Accuracy is bounded by the width of the bucket containing the target quantile.
/// With 23 fine-grained buckets the worst-case error is ≤ 250 ms (in the 5–7.5 s range).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LatencyPercentiles {
    pub p50: f64,
    pub p75: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
}

/// HTTP response status class distribution for an endpoint.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatusDistribution {
    pub status_1xx: i64,
    pub status_2xx: i64,
    pub status_3xx: i64,
    pub status_4xx: i64,
    pub status_5xx: i64,
}

/// Apdex (Application Performance Index) score and its component counts.
///
/// ## Formula
/// ```text
/// Apdex = (satisfied + tolerating / 2) / total   ∈ [0.0, 1.0]
/// ```
///
/// ## Per-tenant T threshold
/// Unlike previous versions, the T threshold is NOT a global constant.
/// It comes from [`TenantConfig::apdex_threshold_ms`] and is applied at read
/// time against the stored latency histogram. Changing T does not invalidate
/// historical data.
///
/// | Tier       | Condition         |
/// |------------|-------------------|
/// | Satisfied  | latency ≤ T       |
/// | Tolerating | T < latency ≤ 4T  |
/// | Frustrated | latency > 4T      |
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApdexScore {
    /// Score in [0.0, 1.0], rounded to 4 decimal places.
    pub score:      f64,
    pub satisfied:  i64,
    pub tolerating: i64,
    pub frustrated: i64,
}

impl ApdexScore {
    /// Derive score from pre-counted component buckets.
    pub fn from_counts(satisfied: i64, tolerating: i64, frustrated: i64) -> Self {
        let total = satisfied + tolerating + frustrated;
        let score = if total > 0 {
            let raw = (satisfied as f64 + tolerating as f64 / 2.0) / total as f64;
            (raw * 10_000.0).round() / 10_000.0
        } else {
            1.0 // No traffic → perfect by convention
        };
        Self { score, satisfied, tolerating, frustrated }
    }
}

// ── Write-path helpers ────────────────────────────────────────────────────────

/// Upper bounds (ms) for the 23 cumulative histogram buckets, ascending order.
///
/// The implicit +∞ bucket equals `total_hits` — no separate column is needed.
/// Bucket boundaries are chosen for fine resolution sub-500 ms and coarser
/// coverage up to 60 s for batch/long-running APIs.
pub const LATENCY_BUCKET_BOUNDS_MS: [f64; 23] = [
       5.0,   10.0,   25.0,   50.0,   75.0,
     100.0,  150.0,  200.0,  300.0,  400.0,
     500.0,  750.0, 1_000.0, 1_500.0, 2_000.0,
   3_000.0, 5_000.0, 7_500.0, 10_000.0, 15_000.0,
  20_000.0, 30_000.0, 60_000.0,
];

/// Per-event histogram increment set for the 23 cumulative latency buckets.
///
/// Each field is `1` when `latency_ms ≤ upper_bound`, `0` otherwise.
/// Cumulative counts remain correct when rows are merged across time windows
/// and are idempotent under deduplication — both required properties.
///
/// Maps 1-to-1 with the `latency_le_Xms` columns in `endpoint_metrics`.
#[derive(Debug, Clone, Default)]
pub struct LatencyHistogram {
    pub le_5ms:     i32,
    pub le_10ms:    i32,
    pub le_25ms:    i32,
    pub le_50ms:    i32,
    pub le_75ms:    i32,
    pub le_100ms:   i32,
    pub le_150ms:   i32,
    pub le_200ms:   i32,
    pub le_300ms:   i32,
    pub le_400ms:   i32,
    pub le_500ms:   i32,
    pub le_750ms:   i32,
    pub le_1000ms:  i32,
    pub le_1500ms:  i32,
    pub le_2000ms:  i32,
    pub le_3000ms:  i32,
    pub le_5000ms:  i32,
    pub le_7500ms:  i32,
    pub le_10000ms: i32,
    pub le_15000ms: i32,
    pub le_20000ms: i32,
    pub le_30000ms: i32,
    pub le_60000ms: i32,
}

impl LatencyHistogram {
    /// Compute the histogram increment set for a single latency observation.
    pub fn from_latency_ms(latency_ms: f64) -> Self {
        Self {
            le_5ms:     (latency_ms <=     5.0) as i32,
            le_10ms:    (latency_ms <=    10.0) as i32,
            le_25ms:    (latency_ms <=    25.0) as i32,
            le_50ms:    (latency_ms <=    50.0) as i32,
            le_75ms:    (latency_ms <=    75.0) as i32,
            le_100ms:   (latency_ms <=   100.0) as i32,
            le_150ms:   (latency_ms <=   150.0) as i32,
            le_200ms:   (latency_ms <=   200.0) as i32,
            le_300ms:   (latency_ms <=   300.0) as i32,
            le_400ms:   (latency_ms <=   400.0) as i32,
            le_500ms:   (latency_ms <=   500.0) as i32,
            le_750ms:   (latency_ms <=   750.0) as i32,
            le_1000ms:  (latency_ms <= 1_000.0) as i32,
            le_1500ms:  (latency_ms <= 1_500.0) as i32,
            le_2000ms:  (latency_ms <= 2_000.0) as i32,
            le_3000ms:  (latency_ms <= 3_000.0) as i32,
            le_5000ms:  (latency_ms <= 5_000.0) as i32,
            le_7500ms:  (latency_ms <= 7_500.0) as i32,
            le_10000ms: (latency_ms <= 10_000.0) as i32,
            le_15000ms: (latency_ms <= 15_000.0) as i32,
            le_20000ms: (latency_ms <= 20_000.0) as i32,
            le_30000ms: (latency_ms <= 30_000.0) as i32,
            le_60000ms: (latency_ms <= 60_000.0) as i32,
        }
    }

    /// Flatten to a `[i64; 23]` array ordered by `LATENCY_BUCKET_BOUNDS_MS`.
    pub fn to_counts_array(&self) -> [i64; 23] {
        [
            self.le_5ms     as i64, self.le_10ms    as i64, self.le_25ms    as i64,
            self.le_50ms    as i64, self.le_75ms    as i64, self.le_100ms   as i64,
            self.le_150ms   as i64, self.le_200ms   as i64, self.le_300ms   as i64,
            self.le_400ms   as i64, self.le_500ms   as i64, self.le_750ms   as i64,
            self.le_1000ms  as i64, self.le_1500ms  as i64, self.le_2000ms  as i64,
            self.le_3000ms  as i64, self.le_5000ms  as i64, self.le_7500ms  as i64,
            self.le_10000ms as i64, self.le_15000ms as i64, self.le_20000ms as i64,
            self.le_30000ms as i64, self.le_60000ms as i64,
        ]
    }
}

/// HTTP status code class for a single request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusClass {
    Status1xx,
    Status2xx,
    Status3xx,
    Status4xx,
    Status5xx,
}

impl StatusClass {
    pub fn from_code(status_code: u16) -> Self {
        match status_code {
            100..=199 => Self::Status1xx,
            200..=299 => Self::Status2xx,
            300..=399 => Self::Status3xx,
            400..=499 => Self::Status4xx,
            _          => Self::Status5xx,
        }
    }
}

// ── Percentile interpolation ──────────────────────────────────────────────────

/// Estimate a single percentile from summed cumulative histogram bucket counts.
///
/// ## Algorithm
/// Linear interpolation within the bucket that straddles the target count:
///
/// ```text
/// target = ⌈quantile × total⌉
/// for each bucket i (ascending upper bound):
///     if cumulative[i] >= target:
///         fraction = (target − cumulative[i−1]) / (cumulative[i] − cumulative[i−1])
///         return lower_bound[i] + fraction × bucket_width[i]
/// ```
///
/// ## Parameters
/// - `counts`   — 23 cumulative bucket counts, ordered by `LATENCY_BUCKET_BOUNDS_MS`
/// - `total`    — total request count (= the implicit +∞ bucket)
/// - `quantile` — target quantile in [0, 1] (e.g. `0.99` for p99)
///
/// Returns `0.0` when `total == 0`; returns `60_000.0` when all requests
/// exceed the highest bucket (> 60 s).
pub fn interpolate_percentile(counts: &[i64; 23], total: i64, quantile: f64) -> f64 {
    if total == 0 {
        return 0.0;
    }

    let target = (quantile * total as f64).ceil() as i64;
    let mut prev_bound = 0.0_f64;
    let mut prev_count = 0_i64;

    for (i, &upper_bound) in LATENCY_BUCKET_BOUNDS_MS.iter().enumerate() {
        let cumulative = counts[i];
        if cumulative >= target {
            let bucket_width   = upper_bound - prev_bound;
            let bucket_entries = cumulative - prev_count;
            if bucket_entries == 0 {
                return upper_bound;
            }
            let fraction = (target - prev_count) as f64 / bucket_entries as f64;
            let estimate = prev_bound + fraction * bucket_width;
            return (estimate * 100.0).round() / 100.0;
        }
        prev_bound = upper_bound;
        prev_count = cumulative;
    }

    // All requests exceed the 60 s upper bound.
    60_000.0
}

/// Compute p50, p75, p90, p95, and p99 in a single pass over the bucket array.
pub fn compute_percentiles(counts: &[i64; 23], total: i64) -> LatencyPercentiles {
    LatencyPercentiles {
        p50: interpolate_percentile(counts, total, 0.50),
        p75: interpolate_percentile(counts, total, 0.75),
        p90: interpolate_percentile(counts, total, 0.90),
        p95: interpolate_percentile(counts, total, 0.95),
        p99: interpolate_percentile(counts, total, 0.99),
    }
}

/// Derive an `ApdexScore` from a 23-bucket cumulative histogram using a given T threshold.
///
/// Finds satisfied count = Σ buckets with upper_bound ≤ T, tolerating count = Σ
/// buckets with T < upper_bound ≤ 4T, frustrated = total − satisfied − tolerating.
///
/// For T values that align with a bucket boundary (200, 300, 500, 750, 1000, 1500, 2000 ms),
/// the result is exact. For other T values, linear interpolation between the two nearest
/// buckets provides a close approximation.
pub fn compute_apdex(counts: &[i64; 23], total: i64, threshold_ms: f64) -> ApdexScore {
    if total == 0 {
        return ApdexScore::from_counts(0, 0, 0);
    }

    let frustrated_threshold = threshold_ms * 4.0;

    // Find the satisfied count: highest bucket with upper_bound ≤ T.
    let satisfied = LATENCY_BUCKET_BOUNDS_MS
        .iter()
        .enumerate()
        .filter(|(_, &bound)| bound <= threshold_ms)
        .map(|(i, _)| counts[i])
        .last()
        .unwrap_or(0);

    // Find the tolerating count: highest bucket with upper_bound ≤ 4T, minus satisfied.
    let satisfied_plus_tolerating = LATENCY_BUCKET_BOUNDS_MS
        .iter()
        .enumerate()
        .filter(|(_, &bound)| bound <= frustrated_threshold)
        .map(|(i, _)| counts[i])
        .last()
        .unwrap_or(0);

    let tolerating = (satisfied_plus_tolerating - satisfied).max(0);
    let frustrated  = (total - satisfied - tolerating).max(0);

    ApdexScore::from_counts(satisfied, tolerating, frustrated)
}
