use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, FromQueryResult, Statement,
    TransactionTrait,
};
use serde::Deserialize;

use crate::domain::ingest::MetricsEvent;
use crate::domain::metrics::{ApiMetricsEntry, EndpointStat, OverallStats, TimeSeriesEntry};
use crate::error::app_error::AppError;

/// Maximum rows returnable in a single query (safety cap).
const MAX_LIMIT: i64 = 1000;

// ── Trait ─────────────────────────────────────────────────────────────────────

/// Metrics repository — all methods return strongly-typed domain structs.
///
/// The implementation detail (SeaORM / raw SQL) is hidden behind this trait.
/// Services import the trait only; the concrete type is wired at startup.
#[async_trait]
pub trait MetricsRepository: Send + Sync {
    /// Upsert one time-bucket row — called by the consumer on every processed event.
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

    /// Idempotently process a metrics event in a single atomic PostgreSQL transaction.
    ///
    /// The transaction does:
    ///   1. `INSERT INTO processed_metric_events (event_id) ON CONFLICT DO NOTHING`
    ///   2. If the insert was new (`rows_affected == 1`): `UPSERT endpoint_metrics`
    ///   3. `COMMIT`
    ///
    /// This guarantees **exactly-once effect** for each `event_id` within the
    /// deduplication retention window, even under at-least-once RabbitMQ redelivery.
    ///
    /// # Returns
    /// - `Ok(true)`  — event was new; metrics updated
    /// - `Ok(false)` — duplicate `event_id`; safely skipped, no double-count
    /// - `Err`       — transaction failed; caller should NACK for retry
    async fn process_event_idempotently(
        &self,
        event: &MetricsEvent,
    ) -> Result<bool, AppError>;

    /// Delete deduplication records older than `retain_until`.
    ///
    /// Should be called periodically (e.g. daily) to keep the table bounded.
    /// Retention period must exceed the maximum RabbitMQ replay window — see SQL comments.
    async fn cleanup_processed_events(
        &self,
        retain_until: DateTime<Utc>,
    ) -> Result<u64, AppError>;

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

// ── SeaORM implementation ─────────────────────────────────────────────────────

/// PostgreSQL implementation using SeaORM for connection management and
/// raw-SQL queries for aggregate analytics (SeaORM `Statement`).
pub struct SeaOrmMetricsRepository {
    db: DatabaseConnection,
}

impl SeaOrmMetricsRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

// ── Internal query-result types ───────────────────────────────────────────────
// These are never exposed above the repository boundary.

#[derive(Debug, FromQueryResult, Deserialize)]
struct StatsRow {
    total_hits: i64,
    error_hits: i64,
    avg_latency: f64,
    unique_services: i64,
    unique_endpoints: i64,
}

#[derive(Debug, FromQueryResult, Deserialize)]
struct EndpointRow {
    service_name: String,
    endpoint: String,
    method: String,
    total_hits: i64,
    avg_latency: f64,
    error_hits: i64,
}

#[derive(Debug, FromQueryResult, Deserialize)]
struct TimeSeriesRow {
    service_name: String,
    endpoint: String,
    method: String,
    total_hits: i64,
    error_hits: i64,
    avg_latency: f64,
    min_latency: f64,
    max_latency: f64,
    time_bucket: DateTime<Utc>,
}

#[derive(Debug, FromQueryResult, Deserialize)]
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

#[derive(Debug, FromQueryResult, Deserialize)]
struct CountRow {
    count: i64,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Truncate a timestamp to the start of the hour (time-bucket for metrics).
fn time_bucket(dt: DateTime<Utc>) -> DateTime<Utc> {
    use chrono::Timelike;
    dt.with_minute(0)
        .and_then(|d| d.with_second(0))
        .and_then(|d| d.with_nanosecond(0))
        .unwrap_or(dt)
}

// ── impl MetricsRepository ────────────────────────────────────────────────────

#[async_trait]
impl MetricsRepository for SeaOrmMetricsRepository {

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
        let sql = r#"
            INSERT INTO endpoint_metrics (
                client_id, service_name, endpoint, method,
                total_hits, error_hits, avg_latency, min_latency, max_latency, time_bucket
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (client_id, service_name, endpoint, method, time_bucket)
            DO UPDATE SET
                total_hits  = endpoint_metrics.total_hits  + EXCLUDED.total_hits,
                error_hits  = endpoint_metrics.error_hits  + EXCLUDED.error_hits,
                avg_latency = (
                    (endpoint_metrics.avg_latency * endpoint_metrics.total_hits)
                    + (EXCLUDED.avg_latency * EXCLUDED.total_hits)
                ) / NULLIF(endpoint_metrics.total_hits + EXCLUDED.total_hits, 0),
                min_latency = LEAST(endpoint_metrics.min_latency, EXCLUDED.min_latency),
                max_latency = GREATEST(endpoint_metrics.max_latency, EXCLUDED.max_latency),
                updated_at  = NOW()
        "#;

        self.db
            .execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                sql,
                [
                    client_id.into(),
                    service_name.into(),
                    endpoint.into(),
                    method.into(),
                    total_hits.into(),
                    error_hits.into(),
                    avg_latency.into(),
                    min_latency.into(),
                    max_latency.into(),
                    time_bucket.into(),
                ],
            ))
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn process_event_idempotently(
        &self,
        event: &MetricsEvent,
    ) -> Result<bool, AppError> {
        // Compute time-bucket and error flag before opening the transaction.
        let time_bucket = time_bucket(event.timestamp);
        let error_hits: i32 = if event.status_code >= 400 { 1 } else { 0 };

        // All writes happen inside one atomic transaction:
        //   Step 1 — dedup guard (ON CONFLICT DO NOTHING)
        //   Step 2 — metrics upsert (only if step 1 inserted a new row)
        //   Commit — both or neither
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| AppError::Database(format!("Failed to begin transaction: {e}")))?;

        // Step 1: attempt to claim this event_id.
        let dedup_sql = r#"
            INSERT INTO processed_metric_events (event_id)
            VALUES ($1)
            ON CONFLICT (event_id) DO NOTHING
        "#;
        let dedup_result = txn
            .execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                dedup_sql,
                [event.event_id.clone().into()],
            ))
            .await
            .map_err(|e| AppError::Database(format!("Dedup insert failed: {e}")))?;

        if dedup_result.rows_affected() == 0 {
            // Already processed — commit the no-op and signal duplicate to caller.
            txn.commit()
                .await
                .map_err(|e| AppError::Database(format!("Transaction commit failed: {e}")))?;
            tracing::debug!(
                event_id = %event.event_id,
                "Duplicate MetricsEvent — idempotent skip"
            );
            return Ok(false);
        }

        // Step 2: update analytics — only reached for genuinely new events.
        let upsert_sql = r#"
            INSERT INTO endpoint_metrics (
                client_id, service_name, endpoint, method,
                total_hits, error_hits, avg_latency, min_latency, max_latency, time_bucket
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (client_id, service_name, endpoint, method, time_bucket)
            DO UPDATE SET
                total_hits  = endpoint_metrics.total_hits  + EXCLUDED.total_hits,
                error_hits  = endpoint_metrics.error_hits  + EXCLUDED.error_hits,
                avg_latency = (
                    (endpoint_metrics.avg_latency * endpoint_metrics.total_hits)
                    + (EXCLUDED.avg_latency * EXCLUDED.total_hits)
                ) / NULLIF(endpoint_metrics.total_hits + EXCLUDED.total_hits, 0),
                min_latency = LEAST(endpoint_metrics.min_latency, EXCLUDED.min_latency),
                max_latency = GREATEST(endpoint_metrics.max_latency, EXCLUDED.max_latency),
                updated_at  = NOW()
        "#;
        txn.execute(Statement::from_sql_and_values(
            DbBackend::Postgres,
            upsert_sql,
            [
                event.client_id.clone().into(),
                event.service_name.clone().into(),
                event.endpoint.clone().into(),
                event.method.clone().into(),
                1i32.into(),
                error_hits.into(),
                event.latency_ms.into(),
                event.latency_ms.into(),
                event.latency_ms.into(),
                time_bucket.into(),
            ],
        ))
        .await
        .map_err(|e| AppError::Database(format!("Metrics upsert failed: {e}")))?;

        // Commit — dedup record and metrics update land together or not at all.
        txn.commit()
            .await
            .map_err(|e| AppError::Database(format!("Transaction commit failed: {e}")))?;

        tracing::debug!(
            event_id = %event.event_id,
            client_id = %event.client_id,
            endpoint  = %event.endpoint,
            "MetricsEvent processed"
        );
        Ok(true)
    }

    async fn cleanup_processed_events(
        &self,
        retain_until: DateTime<Utc>,
    ) -> Result<u64, AppError> {
        let sql = "DELETE FROM processed_metric_events WHERE processed_at < $1";
        let result = self
            .db
            .execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                sql,
                [retain_until.into()],
            ))
            .await
            .map_err(|e| AppError::Database(format!("Cleanup failed: {e}")))?;

        let deleted = result.rows_affected();
        tracing::info!(deleted, "Cleaned up expired dedup records");
        Ok(deleted)
    }

    async fn get_overall_stats(
        &self,
        client_id: Option<&str>,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<OverallStats, AppError> {
        let (sql, values) = if let Some(cid) = client_id {
            (
                r#"
                SELECT
                    COALESCE(SUM(total_hits), 0)                                                        AS total_hits,
                    COALESCE(SUM(error_hits), 0)                                                        AS error_hits,
                    COALESCE(SUM(avg_latency * total_hits) / NULLIF(SUM(total_hits), 0), 0)             AS avg_latency,
                    COUNT(DISTINCT service_name)                                                        AS unique_services,
                    COUNT(DISTINCT endpoint)                                                            AS unique_endpoints
                FROM endpoint_metrics
                WHERE client_id = $1
                  AND time_bucket >= $2
                  AND time_bucket <= $3
                "#,
                vec![cid.into(), start_time.into(), end_time.into()],
            )
        } else {
            (
                r#"
                SELECT
                    COALESCE(SUM(total_hits), 0)                                                        AS total_hits,
                    COALESCE(SUM(error_hits), 0)                                                        AS error_hits,
                    COALESCE(SUM(avg_latency * total_hits) / NULLIF(SUM(total_hits), 0), 0)             AS avg_latency,
                    COUNT(DISTINCT service_name)                                                        AS unique_services,
                    COUNT(DISTINCT endpoint)                                                            AS unique_endpoints
                FROM endpoint_metrics
                WHERE time_bucket >= $1
                  AND time_bucket <= $2
                "#,
                vec![start_time.into(), end_time.into()],
            )
        };

        let row = StatsRow::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            values,
        ))
        .one(&self.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::Database("No stats row returned".to_string()))?;

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

        let (sql, values) = match (client_id, start_time) {
            (Some(cid), Some(st)) => (
                r#"
                SELECT service_name, endpoint, method,
                    SUM(total_hits)                                                 AS total_hits,
                    SUM(avg_latency * total_hits) / NULLIF(SUM(total_hits), 0)     AS avg_latency,
                    SUM(error_hits)                                                 AS error_hits
                FROM endpoint_metrics
                WHERE client_id = $1 AND time_bucket >= $2
                GROUP BY service_name, endpoint, method
                ORDER BY total_hits DESC
                LIMIT $3
                "#,
                vec![cid.into(), st.into(), safe_limit.into()],
            ),
            (Some(cid), None) => (
                r#"
                SELECT service_name, endpoint, method,
                    SUM(total_hits)                                                 AS total_hits,
                    SUM(avg_latency * total_hits) / NULLIF(SUM(total_hits), 0)     AS avg_latency,
                    SUM(error_hits)                                                 AS error_hits
                FROM endpoint_metrics
                WHERE client_id = $1
                GROUP BY service_name, endpoint, method
                ORDER BY total_hits DESC
                LIMIT $2
                "#,
                vec![cid.into(), safe_limit.into()],
            ),
            _ => (
                r#"
                SELECT service_name, endpoint, method,
                    SUM(total_hits)                                                 AS total_hits,
                    SUM(avg_latency * total_hits) / NULLIF(SUM(total_hits), 0)     AS avg_latency,
                    SUM(error_hits)                                                 AS error_hits
                FROM endpoint_metrics
                GROUP BY service_name, endpoint, method
                ORDER BY total_hits DESC
                LIMIT $1
                "#,
                vec![safe_limit.into()],
            ),
        };

        let rows = EndpointRow::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            values,
        ))
        .all(&self.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

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

        let (sql, values) = if let Some(cid) = client_id {
            (
                r#"
                SELECT service_name, endpoint, method,
                    SUM(total_hits)                                                 AS total_hits,
                    SUM(error_hits)                                                 AS error_hits,
                    SUM(avg_latency * total_hits) / NULLIF(SUM(total_hits), 0)     AS avg_latency,
                    MIN(min_latency)                                                AS min_latency,
                    MAX(max_latency)                                                AS max_latency,
                    time_bucket
                FROM endpoint_metrics
                WHERE client_id = $1
                  AND time_bucket >= $2
                  AND time_bucket <= $3
                GROUP BY service_name, endpoint, method, time_bucket
                ORDER BY time_bucket DESC
                LIMIT $4
                "#,
                vec![
                    cid.into(),
                    start_time.into(),
                    end_time.into(),
                    safe_limit.into(),
                ],
            )
        } else {
            (
                r#"
                SELECT service_name, endpoint, method,
                    SUM(total_hits)                                                 AS total_hits,
                    SUM(error_hits)                                                 AS error_hits,
                    SUM(avg_latency * total_hits) / NULLIF(SUM(total_hits), 0)     AS avg_latency,
                    MIN(min_latency)                                                AS min_latency,
                    MAX(max_latency)                                                AS max_latency,
                    time_bucket
                FROM endpoint_metrics
                WHERE time_bucket >= $1
                  AND time_bucket <= $2
                GROUP BY service_name, endpoint, method, time_bucket
                ORDER BY time_bucket DESC
                LIMIT $3
                "#,
                vec![start_time.into(), end_time.into(), safe_limit.into()],
            )
        };

        let rows = TimeSeriesRow::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            values,
        ))
        .all(&self.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

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
                time_bucket: r.time_bucket,
            })
            .collect())
    }

    async fn get_client_apis_metrics(
        &self,
        client_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<ApiMetricsEntry>, i64), AppError> {
        // Count distinct (service, endpoint, method) tuples
        let count_sql = r#"
            SELECT COUNT(*) AS count
            FROM (
                SELECT service_name, endpoint, method
                FROM endpoint_metrics
                WHERE client_id = $1
                GROUP BY service_name, endpoint, method
            ) AS subquery
        "#;

        let count_row = CountRow::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres,
            count_sql,
            [client_id.into()],
        ))
        .one(&self.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::Database("Count query returned no row".to_string()))?;

        let total_count = count_row.count;

        // Paginated metrics rows
        let data_sql = r#"
            SELECT service_name, endpoint, method,
                SUM(total_hits)                                                 AS total_hits,
                SUM(error_hits)                                                 AS error_hits,
                SUM(avg_latency * total_hits) / NULLIF(SUM(total_hits), 0)     AS avg_latency,
                MIN(min_latency)                                                AS min_latency,
                MAX(max_latency)                                                AS max_latency
            FROM endpoint_metrics
            WHERE client_id = $1
            GROUP BY service_name, endpoint, method
            ORDER BY total_hits DESC, service_name, endpoint
            LIMIT $2
            OFFSET $3
        "#;

        let rows = ApiMetricsRow::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres,
            data_sql,
            [client_id.into(), limit.into(), offset.into()],
        ))
        .all(&self.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let items = rows
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
