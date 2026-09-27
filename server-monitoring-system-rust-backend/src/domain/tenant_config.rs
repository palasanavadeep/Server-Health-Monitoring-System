//! Tenant configuration domain types.
//!
//! A tenant (client) can configure per-instance metric behaviour:
//! - `apdex_threshold_ms` — the Apdex T value used for score computation
//! - `histogram_profile`  — which subset of the 23 standard buckets to display
//! - `data_retention_days`— how long to keep aggregated metric rows
//! - `daily_ingest_quota` — optional soft cap on events per day
//!
//! These types are pure domain models with no database or framework dependencies.

use serde::{Deserialize, Serialize};

use crate::domain::metrics::{
    compute_apdex, ApdexScore, LatencyPercentiles, LATENCY_BUCKET_BOUNDS_MS,
};

// ── Histogram profile ─────────────────────────────────────────────────────────

/// A named set of latency bucket boundaries used to customise per-tenant
/// percentile display.
///
/// Storage is always the full 23-bucket layout. A profile is purely a
/// *read-time lens* that selects which boundaries to include in API responses.
///
/// Every boundary in `bucket_bounds` must be present in
/// [`LATENCY_BUCKET_BOUNDS_MS`]. This invariant is enforced by the PostgreSQL
/// `CHECK` constraint on the `histogram_profiles` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistogramProfile {
    pub name:          String,
    pub description:   String,
    pub bucket_bounds: Vec<f64>,
}

impl HistogramProfile {
    /// Canonical indices into `LATENCY_BUCKET_BOUNDS_MS` for this profile's
    /// bucket boundaries, in ascending order.
    ///
    /// Used to select the right cumulative counts from the 23-element array
    /// returned by the repository.
    pub fn bucket_indices(&self) -> Vec<usize> {
        self.bucket_bounds
            .iter()
            .filter_map(|&bound| {
                LATENCY_BUCKET_BOUNDS_MS
                    .iter()
                    .position(|&b| (b - bound).abs() < f64::EPSILON)
            })
            .collect()
    }

    /// Compute p50, p75, p90, p95, p99 using only the bucket boundaries defined
    /// in this profile, via linear interpolation.
    ///
    /// This produces the same result as `compute_percentiles` but restricted to
    /// the profile's visible boundaries. For the `standard` 23-bucket profile
    /// the results are identical.
    pub fn compute_percentiles(&self, counts: &[i64; 23], total: i64) -> LatencyPercentiles {
        let profile_counts: Vec<i64> = self.bucket_indices().iter().map(|&i| counts[i]).collect();
        let profile_bounds: Vec<f64> = self.bucket_bounds.clone();

        LatencyPercentiles {
            p50: interpolate_for_profile(&profile_counts, &profile_bounds, total, 0.50),
            p75: interpolate_for_profile(&profile_counts, &profile_bounds, total, 0.75),
            p90: interpolate_for_profile(&profile_counts, &profile_bounds, total, 0.90),
            p95: interpolate_for_profile(&profile_counts, &profile_bounds, total, 0.95),
            p99: interpolate_for_profile(&profile_counts, &profile_bounds, total, 0.99),
        }
    }
}

/// Linear interpolation within a profile-specific bucket set.
///
/// Identical algorithm to `interpolate_percentile` in `metrics.rs`, but
/// operates on arbitrary sorted bounds rather than the fixed 23-element array.
fn interpolate_for_profile(
    counts: &[i64],
    bounds: &[f64],
    total:  i64,
    quantile: f64,
) -> f64 {
    if total == 0 || counts.is_empty() {
        return 0.0;
    }

    let target     = (quantile * total as f64).ceil() as i64;
    let mut prev_bound = 0.0_f64;
    let mut prev_count = 0_i64;

    for (i, &upper_bound) in bounds.iter().enumerate() {
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

    bounds.last().copied().unwrap_or(0.0)
}

// ── Tenant configuration ──────────────────────────────────────────────────────

/// Per-tenant metric configuration loaded from `client_metric_config`.
///
/// Determines how raw stored data is interpreted and presented for a specific
/// client. Fetched once per request via [`TenantConfigCache`] (TTL: 60 s).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TenantConfig {
    pub client_id:            String,
    /// Apdex T threshold in milliseconds. Apdex is derived from the histogram at
    /// read time using this value — changing it does not invalidate history.
    pub apdex_threshold_ms:   f64,
    /// Named histogram profile controlling which percentile boundaries to display.
    pub histogram_profile:    HistogramProfile,
    /// Rows in `endpoint_metrics` older than this are deleted by the nightly job.
    pub data_retention_days:  i32,
    /// Soft daily ingest quota. `None` = unlimited.
    pub daily_ingest_quota:   Option<i64>,
}

impl TenantConfig {
    /// Derive Apdex from the 23-bucket histogram using this tenant's T threshold.
    ///
    /// Delegates to `compute_apdex` in `domain/metrics.rs`.
    pub fn compute_apdex(&self, counts: &[i64; 23], total: i64) -> ApdexScore {
        compute_apdex(counts, total, self.apdex_threshold_ms)
    }

    /// Compute percentiles using this tenant's active histogram profile.
    pub fn compute_percentiles(&self, counts: &[i64; 23], total: i64) -> LatencyPercentiles {
        self.histogram_profile.compute_percentiles(counts, total)
    }
}

// ── Default / fallback ────────────────────────────────────────────────────────

/// The 23-bucket `standard` profile used when a tenant has no custom config.
pub fn default_histogram_profile() -> HistogramProfile {
    HistogramProfile {
        name:        "standard".to_string(),
        description: "23-bucket general-purpose profile".to_string(),
        bucket_bounds: LATENCY_BUCKET_BOUNDS_MS.to_vec(),
    }
}

impl Default for TenantConfig {
    fn default() -> Self {
        Self {
            client_id:           String::new(),
            apdex_threshold_ms:  500.0,
            histogram_profile:   default_histogram_profile(),
            data_retention_days: 90,
            daily_ingest_quota:  None,
        }
    }
}

// ── Update request ────────────────────────────────────────────────────────────

fn deserialize_double_option<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    serde::Deserialize::deserialize(deserializer).map(Some)
}

/// Fields that may be updated via `PUT /api/admin/clients/:id/config`.
/// All fields are optional — only supplied fields are changed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TenantConfigUpdate {
    #[serde(default)]
    pub apdex_threshold_ms:  Option<f64>,
    #[serde(default)]
    pub histogram_profile:   Option<String>,
    #[serde(default)]
    pub data_retention_days: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub daily_ingest_quota:  Option<Option<i64>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_tenant_config_update_omitted_quota() {
        let json = r#"{"apdexThresholdMs": 300.0}"#;
        let update: TenantConfigUpdate = serde_json::from_str(json).unwrap();
        assert_eq!(update.daily_ingest_quota, None);
    }

    #[test]
    fn test_deserialize_tenant_config_update_null_quota_unlimited() {
        let json = r#"{"dailyIngestQuota": null}"#;
        let update: TenantConfigUpdate = serde_json::from_str(json).unwrap();
        assert_eq!(update.daily_ingest_quota, Some(None));
    }

    #[test]
    fn test_deserialize_tenant_config_update_custom_quota() {
        let json = r#"{"dailyIngestQuota": 500000}"#;
        let update: TenantConfigUpdate = serde_json::from_str(json).unwrap();
        assert_eq!(update.daily_ingest_quota, Some(Some(500000)));
    }
}
