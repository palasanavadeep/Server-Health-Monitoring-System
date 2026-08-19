use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;



use crate::dto::response::analytics::{
    ApiMetricsEntryResponse, DashboardResponse, EndpointStatResponse,
    OverallStatsResponse, TimeRange, TimeSeriesEntryResponse,
};
use crate::error::app_error::AppError;
use crate::repository::metrics_repo::MetricsRepository;

/// Analytics service — all methods return strongly-typed response structs.
/// No serde_json::Value is used at any layer boundary.
pub struct AnalyticsService {
    metrics_repository: Arc<dyn MetricsRepository>,
}

impl AnalyticsService {
    pub fn new(metrics_repository: Arc<dyn MetricsRepository>) -> Self {
        Self { metrics_repository }
    }

    /// Parse time filters with defaults (last 24 h).
    fn parse_time_filters(
        start_time: Option<i64>,
        end_time: Option<i64>,
    ) -> (DateTime<Utc>, DateTime<Utc>) {
        let end = end_time
            .and_then(DateTime::from_timestamp_millis)
            .unwrap_or_else(Utc::now);

        let start = start_time
            .and_then(DateTime::from_timestamp_millis)
            .unwrap_or_else(|| Utc::now() - Duration::hours(24));

        (start, end)
    }

    /// Get overall statistics for a time range.
    pub async fn get_overall_stats(
        &self,
        client_id: Option<&str>,
        start_time: Option<i64>,
        end_time: Option<i64>,
    ) -> Result<OverallStatsResponse, AppError> {
        let (start, end) = Self::parse_time_filters(start_time, end_time);

        let stats = self
            .metrics_repository
            .get_overall_stats(client_id, start, end)
            .await?;

        let error_rate = if stats.total_hits > 0 {
            let raw = (stats.error_hits as f64 / stats.total_hits as f64) * 100.0;
            (raw * 100.0).round() / 100.0
        } else {
            0.0
        };

        Ok(OverallStatsResponse {
            total_hits: stats.total_hits,
            error_hits: stats.error_hits,
            success_hits: stats.total_hits - stats.error_hits,
            error_rate,
            avg_latency: stats.avg_latency,
            unique_services: stats.unique_services,
            unique_endpoints: stats.unique_endpoints,
            time_range: TimeRange { start, end },
        })
    }

    /// Get top endpoints by hit count.
    pub async fn get_top_endpoints(
        &self,
        client_id: Option<&str>,
        limit: i64,
        start_time: Option<i64>,
    ) -> Result<Vec<EndpointStatResponse>, AppError> {
        let parsed_start = start_time.and_then(DateTime::from_timestamp_millis);

        let endpoints = self
            .metrics_repository
            .get_top_endpoints(client_id, limit, parsed_start)
            .await?;

        Ok(endpoints
            .into_iter()
            .map(|ep| {
                let error_rate = if ep.total_hits > 0 {
                    (ep.error_hits as f64 / ep.total_hits as f64) * 100.0
                } else {
                    0.0
                };
                EndpointStatResponse {
                    service_name: ep.service_name,
                    endpoint: ep.endpoint,
                    method: ep.method,
                    total_hits: ep.total_hits,
                    avg_latency: ep.avg_latency,
                    error_hits: ep.error_hits,
                    error_rate,
                }
            })
            .collect())
    }

    /// Get time-series metrics data.
    pub async fn get_time_series(
        &self,
        client_id: Option<&str>,
        start_time: Option<i64>,
        end_time: Option<i64>,
        limit: i64,
    ) -> Result<Vec<TimeSeriesEntryResponse>, AppError> {
        let (start, end) = Self::parse_time_filters(start_time, end_time);

        let metrics = self
            .metrics_repository
            .get_metrics(client_id, start, end, limit)
            .await?;

        Ok(metrics
            .into_iter()
            .map(|m| TimeSeriesEntryResponse {
                service_name: m.service_name,
                endpoint: m.endpoint,
                method: m.method,
                total_hits: m.total_hits,
                error_hits: m.error_hits,
                avg_latency: m.avg_latency,
                min_latency: m.min_latency,
                max_latency: m.max_latency,
                time_bucket: m.time_bucket,
            })
            .collect())
    }

    /// Get paginated client API metrics.
    pub async fn get_client_apis_metrics(
        &self,
        client_id: &str,
        page: i64,
        limit: i64,
    ) -> Result<(Vec<ApiMetricsEntryResponse>, i64, i64, i64), AppError> {
        let parsed_page = page.max(1);
        let parsed_limit = limit.clamp(1, 100);
        let offset = (parsed_page - 1) * parsed_limit;

        let (rows, total_count) = self
            .metrics_repository
            .get_client_apis_metrics(client_id, parsed_limit, offset)
            .await?;

        let _total_pages = if parsed_limit > 0 {
            (total_count as f64 / parsed_limit as f64).ceil() as i64
        } else {
            0
        };

        let items: Vec<ApiMetricsEntryResponse> = rows
            .into_iter()
            .map(|r| {
                let error_rate = if r.total_hits > 0 {
                    ((r.error_hits as f64 / r.total_hits as f64) * 10_000.0).round() / 100.0
                } else {
                    0.0
                };
                ApiMetricsEntryResponse {
                    service_name: r.service_name,
                    endpoint: r.endpoint,
                    method: r.method,
                    total_hits: r.total_hits,
                    error_hits: r.error_hits,
                    success_hits: r.total_hits - r.error_hits,
                    error_rate,
                    avg_latency: r.avg_latency,
                    min_latency: r.min_latency,
                    max_latency: r.max_latency,
                }
            })
            .collect();

        Ok((items, total_count, parsed_page, parsed_limit))
    }

    /// Get all dashboard data in a single parallel fetch.
    pub async fn get_dashboard(
        &self,
        client_id: Option<&str>,
        start_time: Option<i64>,
        end_time: Option<i64>,
    ) -> DashboardResponse {
        let stats_fut = self.get_overall_stats(client_id, start_time, end_time);
        let top_fut = self.get_top_endpoints(client_id, 5, start_time);
        let ts_fut = self.get_time_series(client_id, start_time, end_time, 24);

        let (stats_res, top_res, ts_res) = tokio::join!(stats_fut, top_fut, ts_fut);

        DashboardResponse {
            stats: stats_res.ok(),
            top_endpoints: top_res.ok(),
            recent_activity: ts_res.ok(),
        }
    }
}
