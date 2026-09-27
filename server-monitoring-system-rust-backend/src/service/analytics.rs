use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};

use crate::domain::metrics::ServiceHealthStatus;
use crate::domain::tenant_config::TenantConfig;
use crate::dto::response::analytics::{
    ApiMetricsResponse, DashboardResponse, EndpointMetricsResponse, EndpointStatResponse,
    OverallStatsResponse, ServiceEndpointStatResponse, ServiceHealthResponse,
    ServiceMetricsResponse, ServiceSummaryResponse, TimeRange, TimeSeriesEntryResponse,
};
use crate::error::app_error::AppError;
use crate::repository::metrics_repo::MetricsRepository;

/// Analytics service — orchestrates data retrieval and maps domain structs to
/// response DTOs.
///
/// No SQL, no serialization logic lives here. Those concerns belong to the
/// repository and DTO layers respectively. Per-tenant configuration (Apdex T,
/// histogram profile) is passed through from the caller so that this service
/// stays free of caching and configuration concerns.
pub struct AnalyticsService {
    repository: Arc<dyn MetricsRepository>,
}

impl AnalyticsService {
    pub fn new(repository: Arc<dyn MetricsRepository>) -> Self {
        Self { repository }
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Parse millisecond-epoch timestamps into `DateTime<Utc>`, defaulting to
    /// the last 24 hours when either bound is absent.
    fn resolve_time_range(
        start_ms: Option<i64>,
        end_ms:   Option<i64>,
    ) -> (DateTime<Utc>, DateTime<Utc>) {
        let end   = end_ms.and_then(DateTime::from_timestamp_millis).unwrap_or_else(Utc::now);
        let start = start_ms
            .and_then(DateTime::from_timestamp_millis)
            .unwrap_or_else(|| Utc::now() - Duration::hours(24));
        (start, end)
    }

    /// Error rate as a percentage rounded to 2 decimal places.
    fn error_rate(error_hits: i64, total_hits: i64) -> f64 {
        if total_hits == 0 {
            return 0.0;
        }
        ((error_hits as f64 / total_hits as f64) * 10_000.0).round() / 100.0
    }

    /// Throughput in requests per minute over a time window.
    fn throughput_rpm(total_hits: i64, window_minutes: f64) -> f64 {
        if window_minutes <= 0.0 {
            return 0.0;
        }
        (total_hits as f64 / window_minutes * 100.0).round() / 100.0
    }

    // ── Public API ────────────────────────────────────────────────────────────

    /// Overall statistics across all endpoints for a time window.
    pub async fn get_overall_stats(
        &self,
        client_id: Option<&str>,
        start_ms:  Option<i64>,
        end_ms:    Option<i64>,
    ) -> Result<OverallStatsResponse, AppError> {
        let (start, end) = Self::resolve_time_range(start_ms, end_ms);
        let stats = self.repository.get_overall_stats(client_id, start, end).await?;

        Ok(OverallStatsResponse {
            total_hits:       stats.total_hits,
            error_hits:       stats.error_hits,
            success_hits:     stats.total_hits - stats.error_hits,
            error_rate:       Self::error_rate(stats.error_hits, stats.total_hits),
            avg_latency:      stats.avg_latency,
            unique_services:  stats.unique_services,
            unique_endpoints: stats.unique_endpoints,
            time_range:       TimeRange { start, end },
        })
    }

    /// Top-N endpoints ranked by total hit count.
    ///
    /// Both `start_ms` and `end_ms` are optional. When both are supplied the
    /// query is bounded — this prevents dashboard top-endpoints being
    /// inconsistent with the stats block which is always time-bounded.
    pub async fn get_top_endpoints(
        &self,
        client_id: Option<&str>,
        limit:     i64,
        start_ms:  Option<i64>,
        end_ms:    Option<i64>,
    ) -> Result<Vec<EndpointStatResponse>, AppError> {
        let start = start_ms.and_then(DateTime::from_timestamp_millis);
        let end   = end_ms.and_then(DateTime::from_timestamp_millis);
        let endpoints = self.repository.get_top_endpoints(client_id, limit, start, end).await?;

        Ok(endpoints.into_iter().map(|ep| EndpointStatResponse {
            service_name: ep.service_name,
            endpoint:     ep.endpoint,
            method:       ep.method,
            total_hits:   ep.total_hits,
            avg_latency:  ep.avg_latency,
            error_hits:   ep.error_hits,
            error_rate:   Self::error_rate(ep.error_hits, ep.total_hits),
        }).collect())
    }

    /// Time-series metrics grouped by (endpoint, time_bucket).
    pub async fn get_time_series(
        &self,
        client_id: Option<&str>,
        start_ms:  Option<i64>,
        end_ms:    Option<i64>,
        limit:     i64,
    ) -> Result<Vec<TimeSeriesEntryResponse>, AppError> {
        let (start, end) = Self::resolve_time_range(start_ms, end_ms);
        let rows = self.repository.get_time_series(client_id, start, end, limit).await?;

        Ok(rows.into_iter().map(|r| TimeSeriesEntryResponse {
            service_name: r.service_name,
            endpoint:     r.endpoint,
            method:       r.method,
            total_hits:   r.total_hits,
            error_hits:   r.error_hits,
            avg_latency:  r.avg_latency,
            min_latency:  r.min_latency,
            max_latency:  r.max_latency,
            time_bucket:  r.time_bucket,
        }).collect())
    }

    /// Paginated endpoint metrics table including percentiles, Apdex, and
    /// status distribution. Uses the tenant's config for Apdex T and profile.
    pub async fn get_endpoint_metrics_page(
        &self,
        client_id:     &str,
        tenant_config: &TenantConfig,
        page:          i64,
        limit:         i64,
    ) -> Result<(Vec<ApiMetricsResponse>, i64, i64, i64), AppError> {
        let page   = page.max(1);
        let limit  = limit.clamp(1, 100);
        let offset = (page - 1) * limit;

        let (rows, total) = self
            .repository
            .get_endpoint_metrics_page(client_id, tenant_config, limit, offset)
            .await?;

        let items = rows.into_iter().map(|r| ApiMetricsResponse {
            service_name:        r.service_name,
            endpoint:            r.endpoint,
            method:              r.method,
            total_hits:          r.total_hits,
            error_hits:          r.error_hits,
            success_hits:        r.total_hits - r.error_hits,
            error_rate:          Self::error_rate(r.error_hits, r.total_hits),
            avg_latency:         r.avg_latency,
            min_latency:         r.min_latency,
            max_latency:         r.max_latency,
            percentiles:         r.metrics.percentiles,
            status_distribution: r.metrics.status_distribution,
            apdex:               r.metrics.apdex,
            throughput_rpm:      r.metrics.throughput_rpm,
        }).collect();

        Ok((items, total, page, limit))
    }

    /// Full computed metrics for a single endpoint over a specified time range.
    pub async fn get_endpoint_metrics(
        &self,
        client_id:     &str,
        service_name:  &str,
        endpoint:      &str,
        method:        &str,
        tenant_config: &TenantConfig,
        start_ms:      Option<i64>,
        end_ms:        Option<i64>,
    ) -> Result<EndpointMetricsResponse, AppError> {
        let (start, end) = Self::resolve_time_range(start_ms, end_ms);

        let metrics = self
            .repository
            .get_endpoint_metrics(
                client_id, service_name, endpoint, method,
                tenant_config, start, end,
            )
            .await?;

        Ok(EndpointMetricsResponse {
            percentiles:         metrics.percentiles,
            status_distribution: metrics.status_distribution,
            apdex:               metrics.apdex,
            throughput_rpm:      metrics.throughput_rpm,
            time_range:          TimeRange { start, end },
        })
    }

    /// Composite dashboard snapshot — three queries run in parallel.
    ///
    /// `get_top_endpoints` now passes both bounds so the top-endpoints list
    /// is consistent with the stats block (same time window).
    pub async fn get_dashboard(
        &self,
        client_id: Option<&str>,
        start_ms:  Option<i64>,
        end_ms:    Option<i64>,
    ) -> DashboardResponse {
        let (stats_res, top_res, ts_res) = tokio::join!(
            self.get_overall_stats(client_id, start_ms, end_ms),
            self.get_top_endpoints(client_id, 5, start_ms, end_ms),   // F8: both bounds passed
            self.get_time_series(client_id, start_ms, end_ms, 168),
        );

        if let Err(ref e) = stats_res {
            tracing::error!(error = %e, "get_dashboard: failed to fetch overall stats");
        }
        if let Err(ref e) = top_res {
            tracing::error!(error = %e, "get_dashboard: failed to fetch top endpoints");
        }
        if let Err(ref e) = ts_res {
            tracing::error!(error = %e, "get_dashboard: failed to fetch time series");
        }

        DashboardResponse {
            stats:           stats_res.ok(),
            top_endpoints:   top_res.ok(),
            recent_activity: ts_res.ok(),
        }
    }

    /// Health overview for all services in a tenant — fleet view.
    ///
    /// One row per service; health status is derived from Apdex + error rate.
    pub async fn get_services_health(
        &self,
        client_id:     &str,
        tenant_config: &TenantConfig,
        start_ms:      Option<i64>,
        end_ms:        Option<i64>,
    ) -> Result<Vec<ServiceHealthResponse>, AppError> {
        let (start, end) = Self::resolve_time_range(start_ms, end_ms);
        let window_minutes = ((end - start).num_seconds() as f64 / 60.0).max(1.0);

        let summaries = self.repository
            .get_all_services_summary(client_id, start, end)
            .await?;

        Ok(summaries.into_iter().map(|s| {
            let apdex      = tenant_config.compute_apdex(&s.histogram_counts, s.total_hits);
            let percentiles = tenant_config.compute_percentiles(&s.histogram_counts, s.total_hits);
            let error_rate = Self::error_rate(s.error_hits, s.total_hits);
            let status     = ServiceHealthStatus::classify(apdex.score, error_rate);

            ServiceHealthResponse {
                service_name:    s.service_name,
                total_hits:      s.total_hits,
                error_hits:      s.error_hits,
                success_hits:    s.total_hits - s.error_hits,
                error_rate,
                avg_latency:     s.avg_latency,
                p99_latency:     percentiles.p99,
                apdex:           apdex.score,
                throughput_rpm:  Self::throughput_rpm(s.total_hits, window_minutes),
                endpoint_count:  s.endpoint_count,
                status,
                status_distribution: s.status_distribution,
            }
        }).collect())
    }

    /// Full computed metrics for a single named service.
    ///
    /// Runs summary + endpoint-list queries in parallel, then merges.
    pub async fn get_service_metrics(
        &self,
        client_id:     &str,
        service_name:  &str,
        tenant_config: &TenantConfig,
        start_ms:      Option<i64>,
        end_ms:        Option<i64>,
    ) -> Result<ServiceMetricsResponse, AppError> {
        let (start, end) = Self::resolve_time_range(start_ms, end_ms);
        let window_minutes = ((end - start).num_seconds() as f64 / 60.0).max(1.0);

        let (summary_res, endpoints_res) = tokio::join!(
            self.repository.get_service_summary(client_id, service_name, start, end),
            self.repository.get_service_endpoints(client_id, service_name, tenant_config, start, end),
        );

        let summary_domain = summary_res?.ok_or_else(|| {
            AppError::not_found(format!("Service '{}' has no data in the requested time range", service_name))
        })?;
        let endpoint_stats = endpoints_res?;

        let apdex      = tenant_config.compute_apdex(&summary_domain.histogram_counts, summary_domain.total_hits);
        let percentiles = tenant_config.compute_percentiles(&summary_domain.histogram_counts, summary_domain.total_hits);
        let error_rate = Self::error_rate(summary_domain.error_hits, summary_domain.total_hits);
        let status     = ServiceHealthStatus::classify(apdex.score, error_rate);

        let summary = ServiceSummaryResponse {
            total_hits:          summary_domain.total_hits,
            error_hits:          summary_domain.error_hits,
            success_hits:        summary_domain.total_hits - summary_domain.error_hits,
            error_rate,
            avg_latency:         summary_domain.avg_latency,
            min_latency:         summary_domain.min_latency,
            max_latency:         summary_domain.max_latency,
            throughput_rpm:      Self::throughput_rpm(summary_domain.total_hits, window_minutes),
            percentiles,
            apdex,
            status_distribution: summary_domain.status_distribution,
            status,
        };

        let endpoints = endpoint_stats.into_iter().map(|e| ServiceEndpointStatResponse {
            endpoint:    e.endpoint,
            method:      e.method,
            total_hits:  e.total_hits,
            error_hits:  e.error_hits,
            error_rate:  e.error_rate,
            avg_latency: e.avg_latency,
            p99_latency: e.p99_latency,
            apdex_score: e.apdex_score,
        }).collect();

        Ok(ServiceMetricsResponse {
            service_name: service_name.to_string(),
            summary,
            endpoints,
            time_range: TimeRange { start, end },
        })
    }
}
