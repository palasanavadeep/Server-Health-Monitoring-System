use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::postgres::PgPool;

use crate::domain::metrics::{
    ApiMetricsEntry, EndpointStat, OverallStats, TimeSeriesEntry,
};
use crate::error::app_error::AppError;

const MAX_LIMIT: i64 = 1000;

/// Metrics repository trait — all methods return strongly-typed structs.
#[async_trait]
pub trait MetricsRepository: Send + Sync {
    async fn upsert_endpoint_metrics(
        &self,
        client_id: &str,
        service_name: &str,
        endpoint: &str,
        method: &str,
        total_hits: i32,
        error_hits: i32,
        avg_latency: f64,
        min_latency: f64,
        max_latency: f64,
        time_bucket: DateTime<Utc>,
    ) -> Result<(), AppError>;

    async fn get_overall_stats(
        &self,
        client_id: Option<&str>,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<OverallStats, AppError>;

    async fn get_top_endpoints(
        &self,
        client_id: Option<&str>,
        limit: i64,
        start_time: Option<DateTime<Utc>>,
    ) -> Result<Vec<EndpointStat>, AppError>;

    async fn get_metrics(
        &self,
        client_id: Option<&str>,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<TimeSeriesEntry>, AppError>;

    async fn get_client_apis_metrics(
        &self,
        client_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<ApiMetricsEntry>, i64), AppError>;
}

/// PostgreSQL implementation of `MetricsRepository`.
pub struct PgMetricsRepository {
    pool: PgPool,
}

impl PgMetricsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

// ── Private SQLx row types ────────────────────────────────────────────────────
// These are internal to the impl and never cross the repository boundary.

#[derive(sqlx::FromRow)]
struct StatsRow {
    total_hits: i64,
    error_hits: i64,
    avg_latency: f64,
    unique_services: i64,
    unique_endpoints: i64,
}

#[derive(sqlx::FromRow)]
struct EndpointRow {
    service_name: String,
    endpoint: String,
    method: String,
    total_hits: i64,
    avg_latency: f64,
    error_hits: i64,
}

#[derive(sqlx::FromRow)]
struct TimeSeriesRow {
    service_name: String,
    endpoint: String,
    method: String,
    total_hits: i64,
    error_hits: i64,
    avg_latency: f64,
    min_latency: f64,
    max_latency: f64,
    time_bucket: chrono::NaiveDateTime,
}

#[derive(sqlx::FromRow)]
struct ApiMetricsRow {
    service_name: String,
    endpoint: String,
    method: String,
    total_hits: i64,
    error_hits: i64,
    avg_latency: f64,
    min_latency: f64,
    max_latency: f64,
}

// ── MetricsRepository impl ────────────────────────────────────────────────────

#[async_trait]
impl MetricsRepository for PgMetricsRepository {
    async fn upsert_endpoint_metrics(
        &self,
        client_id: &str,
        service_name: &str,
        endpoint: &str,
        method: &str,
        total_hits: i32,
        error_hits: i32,
        avg_latency: f64,
        min_latency: f64,
        max_latency: f64,
        time_bucket: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO endpoint_metrics (
                client_id, service_name, endpoint, method, total_hits, error_hits,
                avg_latency, min_latency, max_latency, time_bucket
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (client_id, service_name, endpoint, method, time_bucket)
            DO UPDATE SET
               total_hits = endpoint_metrics.total_hits + EXCLUDED.total_hits,
               error_hits = endpoint_metrics.error_hits + EXCLUDED.error_hits,
               avg_latency = (
                (endpoint_metrics.avg_latency * endpoint_metrics.total_hits) + (EXCLUDED.avg_latency * EXCLUDED.total_hits)
               ) / (endpoint_metrics.total_hits + EXCLUDED.total_hits),
                min_latency = LEAST(endpoint_metrics.min_latency, EXCLUDED.min_latency),
                max_latency = GREATEST(endpoint_metrics.max_latency, EXCLUDED.max_latency),
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(client_id)
        .bind(service_name)
        .bind(endpoint)
        .bind(method)
        .bind(total_hits)
        .bind(error_hits)
        .bind(avg_latency)
        .bind(min_latency)
        .bind(max_latency)
        .bind(time_bucket.naive_utc())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_overall_stats(
        &self,
        client_id: Option<&str>,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<OverallStats, AppError> {
        let row = if let Some(cid) = client_id {
            sqlx::query_as::<_, StatsRow>(
                r#"
                SELECT
                    COALESCE(SUM(total_hits), 0) as total_hits,
                    COALESCE(SUM(error_hits), 0) as error_hits,
                    COALESCE(SUM(avg_latency * total_hits) / NULLIF(SUM(total_hits), 0), 0) as avg_latency,
                    COUNT(DISTINCT service_name) as unique_services,
                    COUNT(DISTINCT endpoint) as unique_endpoints
                FROM endpoint_metrics
                WHERE client_id = $1 AND time_bucket >= $2 AND time_bucket <= $3
                "#,
            )
            .bind(cid)
            .bind(start_time.naive_utc())
            .bind(end_time.naive_utc())
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, StatsRow>(
                r#"
                SELECT
                    COALESCE(SUM(total_hits), 0) as total_hits,
                    COALESCE(SUM(error_hits), 0) as error_hits,
                    COALESCE(SUM(avg_latency * total_hits) / NULLIF(SUM(total_hits), 0), 0) as avg_latency,
                    COUNT(DISTINCT service_name) as unique_services,
                    COUNT(DISTINCT endpoint) as unique_endpoints
                FROM endpoint_metrics
                WHERE time_bucket >= $1 AND time_bucket <= $2
                "#,
            )
            .bind(start_time.naive_utc())
            .bind(end_time.naive_utc())
            .fetch_one(&self.pool)
            .await?
        };

        // Map SQLx row → domain struct (no allocation overhead from JSON)
        Ok(OverallStats {
            total_hits: row.total_hits,
            error_hits: row.error_hits,
            avg_latency: row.avg_latency,
            unique_services: row.unique_services,
            unique_endpoints: row.unique_endpoints,
        })
    }

    async fn get_top_endpoints(
        &self,
        client_id: Option<&str>,
        limit: i64,
        start_time: Option<DateTime<Utc>>,
    ) -> Result<Vec<EndpointStat>, AppError> {
        let safe_limit = limit.clamp(1, MAX_LIMIT);

        let rows: Vec<EndpointRow> = if let Some(cid) = client_id {
            if let Some(st) = start_time {
                sqlx::query_as::<_, EndpointRow>(
                    r#"
                    SELECT service_name, endpoint, method,
                        SUM(total_hits) as total_hits,
                        SUM(avg_latency * total_hits) / NULLIF(SUM(total_hits), 0) as avg_latency,
                        SUM(error_hits) as error_hits
                    FROM endpoint_metrics
                    WHERE client_id = $1 AND time_bucket >= $2
                    GROUP BY service_name, endpoint, method
                    ORDER BY total_hits DESC
                    LIMIT $3
                    "#,
                )
                .bind(cid)
                .bind(st.naive_utc())
                .bind(safe_limit)
                .fetch_all(&self.pool)
                .await?
            } else {
                sqlx::query_as::<_, EndpointRow>(
                    r#"
                    SELECT service_name, endpoint, method,
                        SUM(total_hits) as total_hits,
                        SUM(avg_latency * total_hits) / NULLIF(SUM(total_hits), 0) as avg_latency,
                        SUM(error_hits) as error_hits
                    FROM endpoint_metrics
                    WHERE client_id = $1
                    GROUP BY service_name, endpoint, method
                    ORDER BY total_hits DESC
                    LIMIT $2
                    "#,
                )
                .bind(cid)
                .bind(safe_limit)
                .fetch_all(&self.pool)
                .await?
            }
        } else {
            sqlx::query_as::<_, EndpointRow>(
                r#"
                SELECT service_name, endpoint, method,
                    SUM(total_hits) as total_hits,
                    SUM(avg_latency * total_hits) / NULLIF(SUM(total_hits), 0) as avg_latency,
                    SUM(error_hits) as error_hits
                FROM endpoint_metrics
                GROUP BY service_name, endpoint, method
                ORDER BY total_hits DESC
                LIMIT $1
                "#,
            )
            .bind(safe_limit)
            .fetch_all(&self.pool)
            .await?
        };

        // Map directly — no intermediate JSON Value
        Ok(rows
            .into_iter()
            .map(|r| EndpointStat {
                service_name: r.service_name,
                endpoint: r.endpoint,
                method: r.method,
                total_hits: r.total_hits,
                avg_latency: r.avg_latency,
                error_hits: r.error_hits,
            })
            .collect())
    }

    async fn get_metrics(
        &self,
        client_id: Option<&str>,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<TimeSeriesEntry>, AppError> {
        let safe_limit = limit.clamp(1, MAX_LIMIT);

        let rows: Vec<TimeSeriesRow> = if let Some(cid) = client_id {
            sqlx::query_as::<_, TimeSeriesRow>(
                r#"
                SELECT service_name, endpoint, method,
                    SUM(total_hits) as total_hits,
                    SUM(error_hits) as error_hits,
                    SUM(avg_latency * total_hits) / NULLIF(SUM(total_hits), 0) as avg_latency,
                    MIN(min_latency) as min_latency,
                    MAX(max_latency) as max_latency,
                    time_bucket
                FROM endpoint_metrics
                WHERE client_id = $1 AND time_bucket >= $2 AND time_bucket <= $3
                GROUP BY service_name, endpoint, method, time_bucket
                ORDER BY time_bucket DESC
                LIMIT $4
                "#,
            )
            .bind(cid)
            .bind(start_time.naive_utc())
            .bind(end_time.naive_utc())
            .bind(safe_limit)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, TimeSeriesRow>(
                r#"
                SELECT service_name, endpoint, method,
                    SUM(total_hits) as total_hits,
                    SUM(error_hits) as error_hits,
                    SUM(avg_latency * total_hits) / NULLIF(SUM(total_hits), 0) as avg_latency,
                    MIN(min_latency) as min_latency,
                    MAX(max_latency) as max_latency,
                    time_bucket
                FROM endpoint_metrics
                WHERE time_bucket >= $1 AND time_bucket <= $2
                GROUP BY service_name, endpoint, method, time_bucket
                ORDER BY time_bucket DESC
                LIMIT $3
                "#,
            )
            .bind(start_time.naive_utc())
            .bind(end_time.naive_utc())
            .bind(safe_limit)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows
            .into_iter()
            .map(|r| TimeSeriesEntry {
                service_name: r.service_name,
                endpoint: r.endpoint,
                method: r.method,
                total_hits: r.total_hits,
                error_hits: r.error_hits,
                avg_latency: r.avg_latency,
                min_latency: r.min_latency,
                max_latency: r.max_latency,
                time_bucket: r.time_bucket.and_utc(),
            })
            .collect())
    }

    async fn get_client_apis_metrics(
        &self,
        client_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<ApiMetricsEntry>, i64), AppError> {
        let count_row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM (
                SELECT service_name, endpoint, method
                FROM endpoint_metrics
                WHERE client_id = $1
                GROUP BY service_name, endpoint, method
            ) AS temp
            "#,
        )
        .bind(client_id)
        .fetch_one(&self.pool)
        .await?;

        let total_count = count_row.0;

        let rows: Vec<ApiMetricsRow> = sqlx::query_as::<_, ApiMetricsRow>(
            r#"
            SELECT service_name, endpoint, method,
                SUM(total_hits) as total_hits,
                SUM(error_hits) as error_hits,
                SUM(avg_latency * total_hits) / NULLIF(SUM(total_hits), 0) as avg_latency,
                MIN(min_latency) as min_latency,
                MAX(max_latency) as max_latency
            FROM endpoint_metrics
            WHERE client_id = $1
            GROUP BY service_name, endpoint, method
            ORDER BY total_hits DESC, service_name, endpoint
            LIMIT $2
            OFFSET $3
            "#,
        )
        .bind(client_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let items: Vec<ApiMetricsEntry> = rows
            .into_iter()
            .map(|r| ApiMetricsEntry {
                service_name: r.service_name,
                endpoint: r.endpoint,
                method: r.method,
                total_hits: r.total_hits,
                error_hits: r.error_hits,
                avg_latency: r.avg_latency,
                min_latency: r.min_latency,
                max_latency: r.max_latency,
            })
            .collect();

        Ok((items, total_count))
    }
}
