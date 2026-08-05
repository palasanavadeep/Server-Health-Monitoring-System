use chrono::{Utc, Duration, DateTime};

use crate::errors::app_error::AppError;
use crate::services::processor::metrics_repository::MetricsRepository;

/// AnalyticsService - Aggregates and transforms metrics data.
/// Mirrors Node.js AnalyticsService class.
pub struct AnalyticsService {
    metrics_repository: MetricsRepository,
}

impl AnalyticsService {
    pub fn new(metrics_repository: MetricsRepository) -> Self {
        Self { metrics_repository }
    }

    /// Parse time filters with defaults (last 24h).
    fn parse_time_filters(
        start_time: Option<i64>,
        end_time: Option<i64>,
    ) -> (DateTime<Utc>, DateTime<Utc>) {
        let end = match end_time {
            Some(ts) => {
                chrono::DateTime::from_timestamp_millis(ts)
                    .unwrap_or_else(Utc::now)
            }
            None => Utc::now(),
        };

        let start = match start_time {
            Some(ts) => {
                chrono::DateTime::from_timestamp_millis(ts)
                    .unwrap_or_else(|| Utc::now() - Duration::hours(24))
            }
            None => Utc::now() - Duration::hours(24),
        };

        (start, end)
    }

    /// Get overall stats.
    pub async fn get_overall_stats(
        &self,
        client_id: Option<&str>,
        start_time: Option<i64>,
        end_time: Option<i64>,
    ) -> Result<serde_json::Value, AppError> {
        let (start, end) = Self::parse_time_filters(start_time, end_time);

        let stats = self
            .metrics_repository
            .get_overall_stats(client_id, start, end)
            .await?;

        let total_hits: i64 = stats.get("total_hits").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0);
        let error_hits: i64 = stats.get("error_hits").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0);
        let error_rate = if total_hits > 0 {
            (error_hits as f64 / total_hits as f64) * 100.0
        } else {
            0.0
        };

        Ok(serde_json::json!({
            "totalHits": total_hits,
            "errorHits": error_hits,
            "successHits": total_hits - error_hits,
            "errorRate": (error_rate * 100.0).round() / 100.0,
            "avgLatency": stats.get("avg_latency").and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0),
            "uniqueServices": stats.get("unique_services").and_then(|v| v.as_str()).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0),
            "uniqueEndpoints": stats.get("unique_endpoints").and_then(|v| v.as_str()).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0),
            "timeRange": {
                "start": start.to_rfc3339(),
                "end": end.to_rfc3339(),
            }
        }))
    }

    /// Get top endpoints.
    pub async fn get_top_endpoints(
        &self,
        client_id: Option<&str>,
        limit: i64,
        start_time: Option<i64>,
    ) -> Result<Vec<serde_json::Value>, AppError> {
        let parsed_start = start_time.and_then(chrono::DateTime::from_timestamp_millis);

        let endpoints = self
            .metrics_repository
            .get_top_endpoints(client_id, limit, parsed_start)
            .await?;

        Ok(endpoints
            .into_iter()
            .map(|ep| {
                let total_hits: i64 = ep.get("total_hits").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0);
                let error_hits: i64 = ep.get("error_hits").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0);
                let error_rate = if total_hits > 0 {
                    format!("{:.2}", (error_hits as f64 / total_hits as f64) * 100.0)
                } else {
                    "0.00".to_string()
                };

                serde_json::json!({
                    "serviceName": ep.get("service_name"),
                    "endpoint": ep.get("endpoint"),
                    "method": ep.get("method"),
                    "totalHits": total_hits,
                    "avgLatency": ep.get("avg_latency").and_then(|v| v.as_str()).unwrap_or("0.000"),
                    "errorHits": error_hits,
                    "errorRate": error_rate,
                })
            })
            .collect())
    }

    /// Get time series.
    pub async fn get_time_series(
        &self,
        client_id: Option<&str>,
        start_time: Option<i64>,
        end_time: Option<i64>,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, AppError> {
        let (start, end) = Self::parse_time_filters(start_time, end_time);

        let metrics = self
            .metrics_repository
            .get_metrics(client_id, start, end, limit)
            .await?;

        Ok(metrics
            .into_iter()
            .map(|m| {
                serde_json::json!({
                    "serviceName": m.get("service_name"),
                    "endpoint": m.get("endpoint"),
                    "method": m.get("method"),
                    "totalHits": m.get("total_hits").and_then(|v| v.as_str()).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0),
                    "errorHits": m.get("error_hits").and_then(|v| v.as_str()).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0),
                    "avgLatency": m.get("avg_latency"),
                    "minLatency": m.get("min_latency"),
                    "maxLatency": m.get("max_latency"),
                    "timeBucket": m.get("time_bucket"),
                })
            })
            .collect())
    }

    /// Get paginated client API metrics.
    pub async fn get_client_apis_metrics(
        &self,
        client_id: &str,
        page: i64,
        limit: i64,
    ) -> Result<serde_json::Value, AppError> {
        let parsed_page = page.max(1);
        let parsed_limit = limit.max(1).min(100);
        let offset = (parsed_page - 1) * parsed_limit;

        let (rows, total_count) = self
            .metrics_repository
            .get_client_apis_metrics(client_id, parsed_limit, offset)
            .await?;

        let items: Vec<serde_json::Value> = rows
            .into_iter()
            .map(|row| {
                let total_hits: i64 = row.get("total_hits").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0);
                let error_hits: i64 = row.get("error_hits").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0);
                let error_rate = if total_hits > 0 {
                    ((error_hits as f64 / total_hits as f64) * 10000.0).round() / 100.0
                } else {
                    0.0
                };

                serde_json::json!({
                    "serviceName": row.get("service_name"),
                    "endpoint": row.get("endpoint"),
                    "method": row.get("method"),
                    "totalHits": total_hits,
                    "errorHits": error_hits,
                    "successHits": total_hits - error_hits,
                    "errorRate": error_rate,
                    "avgLatency": row.get("avg_latency").and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0),
                    "minLatency": row.get("min_latency").and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0),
                    "maxLatency": row.get("max_latency").and_then(|v| v.as_str()).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0),
                })
            })
            .collect();

        let total_pages = if parsed_limit > 0 {
            (total_count as f64 / parsed_limit as f64).ceil() as i64
        } else {
            0
        };

        Ok(serde_json::json!({
            "items": items,
            "pagination": {
                "page": parsed_page,
                "limit": parsed_limit,
                "totalCount": total_count,
                "totalPages": total_pages,
            }
        }))
    }
}
