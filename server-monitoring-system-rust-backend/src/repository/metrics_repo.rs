use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, FromQueryResult, Statement, TransactionTrait,
};
use serde::Deserialize;

use crate::domain::ingest::MetricsEvent;
use crate::domain::metrics::{
    ApiMetricsEntry, EndpointMetrics, EndpointStat, LatencyHistogram,
    OverallStats, ServiceSummary, StatusClass, StatusDistribution, TimeSeriesEntry,
};
use crate::domain::tenant_config::TenantConfig;
use crate::error::app_error::AppError;

/// Safety cap on rows returned by a single list query.
const MAX_QUERY_LIMIT: i64 = 1_000;

// ── Repository trait ──────────────────────────────────────────────────────────

/// Data access interface for aggregated endpoint metrics.
///
/// All read methods return strongly typed domain structs. The implementation
/// detail (SQL, SeaORM) is hidden behind this trait; tests can provide an
/// in-memory mock.
#[async_trait]
pub trait MetricsRepository: Send + Sync {
    /// Process a single metrics event inside an atomic PostgreSQL transaction.
    ///
    /// ## Transaction steps
    /// 1. `INSERT INTO processed_metric_events (event_id) ON CONFLICT DO NOTHING`
    /// 2. If new (`rows_affected == 1`): upsert `endpoint_metrics` — all 23
    ///    histogram buckets, status counters, running latency aggregates.
    /// 3. `COMMIT` — both writes land together or not at all.
    ///
    /// ## Returns
    /// - `Ok(true)`  — new event, counters updated
    /// - `Ok(false)` — duplicate `event_id`, safely skipped (idempotent)
    /// - `Err`       — transaction failed; caller should NACK for retry
    async fn process_event(&self, event: &MetricsEvent) -> Result<bool, AppError>;

    /// Delete deduplication records older than `retain_until`.
    async fn delete_expired_dedup_records(
        &self,
        retain_until: DateTime<Utc>,
    ) -> Result<u64, AppError>;

    /// Overall statistics aggregated across all endpoints for a time range.
    async fn get_overall_stats(
        &self,
        client_id: Option<&str>,
        start_time: DateTime<Utc>,
        end_time:   DateTime<Utc>,
    ) -> Result<OverallStats, AppError>;

    /// Top-N endpoints by total hit count.
    async fn get_top_endpoints(
        &self,
        client_id:  Option<&str>,
        limit:      i64,
        start_time: Option<DateTime<Utc>>,
        end_time:   Option<DateTime<Utc>>,
    ) -> Result<Vec<EndpointStat>, AppError>;

    /// Time-series rows grouped by (endpoint, time_bucket).
    async fn get_time_series(
        &self,
        client_id:  Option<&str>,
        start_time: DateTime<Utc>,
        end_time:   DateTime<Utc>,
        limit:      i64,
    ) -> Result<Vec<TimeSeriesEntry>, AppError>;

    /// Paginated per-endpoint metrics including computed performance metrics
    /// (percentiles, Apdex, status distribution, throughput).
    ///
    /// `tenant_config` is used to select the correct Apdex T and histogram
    /// profile for this client's data.
    async fn get_endpoint_metrics_page(
        &self,
        client_id:     &str,
        tenant_config: &TenantConfig,
        limit:         i64,
        offset:        i64,
    ) -> Result<(Vec<ApiMetricsEntry>, i64), AppError>;

    /// Full computed metrics for a single endpoint over a time range.
    async fn get_endpoint_metrics(
        &self,
        client_id:     &str,
        service_name:  &str,
        endpoint:      &str,
        method:        &str,
        tenant_config: &TenantConfig,
        start_time:    DateTime<Utc>,
        end_time:      DateTime<Utc>,
    ) -> Result<EndpointMetrics, AppError>;

    /// Service-level aggregation — one row per service for the fleet overview.
    async fn get_all_services_summary(
        &self,
        client_id:  &str,
        start_time: DateTime<Utc>,
        end_time:   DateTime<Utc>,
    ) -> Result<Vec<ServiceSummary>, AppError>;

    /// Aggregated summary for a single named service.
    async fn get_service_summary(
        &self,
        client_id:    &str,
        service_name: &str,
        start_time:   DateTime<Utc>,
        end_time:     DateTime<Utc>,
    ) -> Result<Option<ServiceSummary>, AppError>;

    /// Per-endpoint stats for a single service (for the detail view endpoint list).
    async fn get_service_endpoints(
        &self,
        client_id:     &str,
        service_name:  &str,
        tenant_config: &TenantConfig,
        start_time:    DateTime<Utc>,
        end_time:      DateTime<Utc>,
    ) -> Result<Vec<crate::domain::metrics::ServiceEndpointStat>, AppError>;

    /// Delete endpoint metric rows older than `retain_until` for a given client.
    async fn delete_client_metrics_before(
        &self,
        client_id:    &str,
        retain_until: DateTime<Utc>,
    ) -> Result<u64, AppError>;
}

// ── PostgreSQL implementation ─────────────────────────────────────────────────

/// SeaORM-backed PostgreSQL implementation of [`MetricsRepository`].
///
/// Uses raw SQL via `sea_orm::Statement` for all aggregate queries.
/// SeaORM is used only for connection management and transaction orchestration.
pub struct SeaOrmMetricsRepository {
    db: DatabaseConnection,
}

impl SeaOrmMetricsRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

// ── Internal query-result structs ─────────────────────────────────────────────
// Mapped directly from raw SQL result sets. Never exposed beyond this module.

#[derive(Debug, FromQueryResult, Deserialize)]
struct OverallStatsRow {
    total_hits:       i64,
    error_hits:       i64,
    avg_latency:      f64,
    unique_services:  i64,
    unique_endpoints: i64,
}

#[derive(Debug, FromQueryResult, Deserialize)]
struct EndpointStatRow {
    service_name: String,
    endpoint:     String,
    method:       String,
    total_hits:   i64,
    avg_latency:  f64,
    error_hits:   i64,
}

#[derive(Debug, FromQueryResult, Deserialize)]
struct TimeSeriesRow {
    service_name: String,
    endpoint:     String,
    method:       String,
    total_hits:   i64,
    error_hits:   i64,
    avg_latency:  f64,
    min_latency:  f64,
    max_latency:  f64,
    time_bucket:  NaiveDateTime,
}

/// Extended row including the full 23-bucket histogram and status counters.
///
/// Apdex columns are no longer stored — derived at read time from the histogram.
#[derive(Debug, FromQueryResult, Deserialize)]
struct EndpointMetricsRow {
    service_name: String,
    endpoint:     String,
    method:       String,
    total_hits:   i64,
    error_hits:   i64,
    avg_latency:  f64,
    min_latency:  f64,
    max_latency:  f64,
    active_buckets: Option<i64>,
    // 23-bucket cumulative histogram
    latency_le_5ms:     i64,
    latency_le_10ms:    i64,
    latency_le_25ms:    i64,
    latency_le_50ms:    i64,
    latency_le_75ms:    i64,
    latency_le_100ms:   i64,
    latency_le_150ms:   i64,
    latency_le_200ms:   i64,
    latency_le_300ms:   i64,
    latency_le_400ms:   i64,
    latency_le_500ms:   i64,
    latency_le_750ms:   i64,
    latency_le_1000ms:  i64,
    latency_le_1500ms:  i64,
    latency_le_2000ms:  i64,
    latency_le_3000ms:  i64,
    latency_le_5000ms:  i64,
    latency_le_7500ms:  i64,
    latency_le_10000ms: i64,
    latency_le_15000ms: i64,
    latency_le_20000ms: i64,
    latency_le_30000ms: i64,
    latency_le_60000ms: i64,
    // Status class counters
    status_1xx: i64,
    status_2xx: i64,
    status_3xx: i64,
    status_4xx: i64,
    status_5xx: i64,
}

#[derive(Debug, FromQueryResult, Deserialize)]
struct CountRow {
    count: i64,
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Truncate a UTC timestamp to the start of the current clock-hour.
///
/// All events within the same hour are aggregated into a single
/// `endpoint_metrics` row via the composite unique key.
fn truncate_to_hour(dt: DateTime<Utc>) -> DateTime<Utc> {
    use chrono::Timelike;
    dt.with_minute(0)
        .and_then(|d| d.with_second(0))
        .and_then(|d| d.with_nanosecond(0))
        .unwrap_or(dt)
}

/// Extract a `[i64; 23]` bucket-count array from a query result row.
fn bucket_counts(row: &EndpointMetricsRow) -> [i64; 23] {
    [
        row.latency_le_5ms,   row.latency_le_10ms,  row.latency_le_25ms,
        row.latency_le_50ms,  row.latency_le_75ms,  row.latency_le_100ms,
        row.latency_le_150ms, row.latency_le_200ms, row.latency_le_300ms,
        row.latency_le_400ms, row.latency_le_500ms, row.latency_le_750ms,
        row.latency_le_1000ms, row.latency_le_1500ms, row.latency_le_2000ms,
        row.latency_le_3000ms, row.latency_le_5000ms, row.latency_le_7500ms,
        row.latency_le_10000ms, row.latency_le_15000ms, row.latency_le_20000ms,
        row.latency_le_30000ms, row.latency_le_60000ms,
    ]
}

/// Build [`EndpointMetrics`] from a query row using the tenant's config.
///
/// - Percentiles are derived from the tenant's active histogram profile.
/// - Apdex is derived from the histogram using the tenant's T threshold.
/// - `time_window_minutes` drives throughput; pass `0.0` for all-time queries.
fn build_endpoint_metrics(
    row:                &EndpointMetricsRow,
    tenant_config:      &TenantConfig,
    time_window_minutes: f64,
) -> EndpointMetrics {
    let counts = bucket_counts(row);

    EndpointMetrics {
        percentiles:         tenant_config.compute_percentiles(&counts, row.total_hits),
        status_distribution: StatusDistribution {
            status_1xx: row.status_1xx,
            status_2xx: row.status_2xx,
            status_3xx: row.status_3xx,
            status_4xx: row.status_4xx,
            status_5xx: row.status_5xx,
        },
        apdex: tenant_config.compute_apdex(&counts, row.total_hits),
        throughput_rpm: if time_window_minutes > 0.0 {
            (row.total_hits as f64 / time_window_minutes * 100.0).round() / 100.0
        } else {
            0.0
        },
    }
}

// ── SQL fragments ─────────────────────────────────────────────────────────────

/// SELECT clause listing all 23 histogram + status SUM columns.
/// Used in both `get_endpoint_metrics_page` and `get_endpoint_metrics` to avoid
/// duplicating the same 28-column list.
const HISTOGRAM_SELECT: &str = "
    SUM(latency_le_5ms)     AS latency_le_5ms,
    SUM(latency_le_10ms)    AS latency_le_10ms,
    SUM(latency_le_25ms)    AS latency_le_25ms,
    SUM(latency_le_50ms)    AS latency_le_50ms,
    SUM(latency_le_75ms)    AS latency_le_75ms,
    SUM(latency_le_100ms)   AS latency_le_100ms,
    SUM(latency_le_150ms)   AS latency_le_150ms,
    SUM(latency_le_200ms)   AS latency_le_200ms,
    SUM(latency_le_300ms)   AS latency_le_300ms,
    SUM(latency_le_400ms)   AS latency_le_400ms,
    SUM(latency_le_500ms)   AS latency_le_500ms,
    SUM(latency_le_750ms)   AS latency_le_750ms,
    SUM(latency_le_1000ms)  AS latency_le_1000ms,
    SUM(latency_le_1500ms)  AS latency_le_1500ms,
    SUM(latency_le_2000ms)  AS latency_le_2000ms,
    SUM(latency_le_3000ms)  AS latency_le_3000ms,
    SUM(latency_le_5000ms)  AS latency_le_5000ms,
    SUM(latency_le_7500ms)  AS latency_le_7500ms,
    SUM(latency_le_10000ms) AS latency_le_10000ms,
    SUM(latency_le_15000ms) AS latency_le_15000ms,
    SUM(latency_le_20000ms) AS latency_le_20000ms,
    SUM(latency_le_30000ms) AS latency_le_30000ms,
    SUM(latency_le_60000ms) AS latency_le_60000ms,
    SUM(status_1xx)         AS status_1xx,
    SUM(status_2xx)         AS status_2xx,
    SUM(status_3xx)         AS status_3xx,
    SUM(status_4xx)         AS status_4xx,
    SUM(status_5xx)         AS status_5xx";

/// Static upsert SQL for `process_event`.
///
/// Extracted as a const so the identical string is not heap-allocated on every
/// ingest event. The SQL never changes at runtime — only the bound parameters do.
const UPSERT_ENDPOINT_METRICS: &str = r#"
    INSERT INTO endpoint_metrics (
        client_id, service_name, endpoint, method,
        total_hits, error_hits,
        avg_latency, min_latency, max_latency,
        latency_le_5ms,  latency_le_10ms,  latency_le_25ms,
        latency_le_50ms, latency_le_75ms,  latency_le_100ms,
        latency_le_150ms,latency_le_200ms, latency_le_300ms,
        latency_le_400ms,latency_le_500ms, latency_le_750ms,
        latency_le_1000ms,latency_le_1500ms,latency_le_2000ms,
        latency_le_3000ms,latency_le_5000ms,latency_le_7500ms,
        latency_le_10000ms,latency_le_15000ms,latency_le_20000ms,
        latency_le_30000ms,latency_le_60000ms,
        status_1xx, status_2xx, status_3xx, status_4xx, status_5xx,
        time_bucket
    )
    VALUES (
        $1, $2, $3, $4,
        1, $5,
        $6, $6, $6,
        $7, $8, $9, $10,$11,$12,
        $13,$14,$15,$16,$17,$18,
        $19,$20,$21,$22,$23,$24,
        $25,$26,$27,$28,$29,
        $30,$31,$32,$33,$34,
        $35
    )
    ON CONFLICT (client_id, service_name, endpoint, method, time_bucket)
    DO UPDATE SET
        total_hits          = endpoint_metrics.total_hits + 1,
        error_hits          = endpoint_metrics.error_hits          + EXCLUDED.error_hits,
        avg_latency         = (
            endpoint_metrics.avg_latency * endpoint_metrics.total_hits
            + EXCLUDED.avg_latency
        ) / NULLIF(endpoint_metrics.total_hits + 1, 0),
        min_latency         = LEAST(endpoint_metrics.min_latency,    EXCLUDED.min_latency),
        max_latency         = GREATEST(endpoint_metrics.max_latency, EXCLUDED.max_latency),
        latency_le_5ms      = endpoint_metrics.latency_le_5ms      + EXCLUDED.latency_le_5ms,
        latency_le_10ms     = endpoint_metrics.latency_le_10ms     + EXCLUDED.latency_le_10ms,
        latency_le_25ms     = endpoint_metrics.latency_le_25ms     + EXCLUDED.latency_le_25ms,
        latency_le_50ms     = endpoint_metrics.latency_le_50ms     + EXCLUDED.latency_le_50ms,
        latency_le_75ms     = endpoint_metrics.latency_le_75ms     + EXCLUDED.latency_le_75ms,
        latency_le_100ms    = endpoint_metrics.latency_le_100ms    + EXCLUDED.latency_le_100ms,
        latency_le_150ms    = endpoint_metrics.latency_le_150ms    + EXCLUDED.latency_le_150ms,
        latency_le_200ms    = endpoint_metrics.latency_le_200ms    + EXCLUDED.latency_le_200ms,
        latency_le_300ms    = endpoint_metrics.latency_le_300ms    + EXCLUDED.latency_le_300ms,
        latency_le_400ms    = endpoint_metrics.latency_le_400ms    + EXCLUDED.latency_le_400ms,
        latency_le_500ms    = endpoint_metrics.latency_le_500ms    + EXCLUDED.latency_le_500ms,
        latency_le_750ms    = endpoint_metrics.latency_le_750ms    + EXCLUDED.latency_le_750ms,
        latency_le_1000ms   = endpoint_metrics.latency_le_1000ms   + EXCLUDED.latency_le_1000ms,
        latency_le_1500ms   = endpoint_metrics.latency_le_1500ms   + EXCLUDED.latency_le_1500ms,
        latency_le_2000ms   = endpoint_metrics.latency_le_2000ms   + EXCLUDED.latency_le_2000ms,
        latency_le_3000ms   = endpoint_metrics.latency_le_3000ms   + EXCLUDED.latency_le_3000ms,
        latency_le_5000ms   = endpoint_metrics.latency_le_5000ms   + EXCLUDED.latency_le_5000ms,
        latency_le_7500ms   = endpoint_metrics.latency_le_7500ms   + EXCLUDED.latency_le_7500ms,
        latency_le_10000ms  = endpoint_metrics.latency_le_10000ms  + EXCLUDED.latency_le_10000ms,
        latency_le_15000ms  = endpoint_metrics.latency_le_15000ms  + EXCLUDED.latency_le_15000ms,
        latency_le_20000ms  = endpoint_metrics.latency_le_20000ms  + EXCLUDED.latency_le_20000ms,
        latency_le_30000ms  = endpoint_metrics.latency_le_30000ms  + EXCLUDED.latency_le_30000ms,
        latency_le_60000ms  = endpoint_metrics.latency_le_60000ms  + EXCLUDED.latency_le_60000ms,
        status_1xx          = endpoint_metrics.status_1xx          + EXCLUDED.status_1xx,
        status_2xx          = endpoint_metrics.status_2xx          + EXCLUDED.status_2xx,
        status_3xx          = endpoint_metrics.status_3xx          + EXCLUDED.status_3xx,
        status_4xx          = endpoint_metrics.status_4xx          + EXCLUDED.status_4xx,
        status_5xx          = endpoint_metrics.status_5xx          + EXCLUDED.status_5xx,
        updated_at          = NOW()
"#;

// ── impl MetricsRepository ────────────────────────────────────────────────────

#[async_trait]
impl MetricsRepository for SeaOrmMetricsRepository {

    async fn process_event(&self, event: &MetricsEvent) -> Result<bool, AppError> {
        // Compute all per-event increments in Rust before opening any transaction.
        let time_bucket = truncate_to_hour(event.timestamp);
        let error_inc   = (event.status_code >= 400) as i32;
        let histogram   = LatencyHistogram::from_latency_ms(event.latency_ms);

        let (inc_1xx, inc_2xx, inc_3xx, inc_4xx, inc_5xx) =
            match StatusClass::from_code(event.status_code) {
                StatusClass::Status1xx => (1i32, 0, 0, 0, 0),
                StatusClass::Status2xx => (0, 1, 0, 0, 0),
                StatusClass::Status3xx => (0, 0, 1, 0, 0),
                StatusClass::Status4xx => (0, 0, 0, 1, 0),
                StatusClass::Status5xx => (0, 0, 0, 0, 1),
            };

        // ── Atomic transaction ─────────────────────────────────────────────────

        let txn = self.db.begin().await
            .map_err(|e| AppError::Database(format!("Begin transaction: {e}")))?;

        // Step 1 — Claim the event_id. Duplicate → rows_affected == 0 → skip.
        let dedup = txn
            .execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "INSERT INTO processed_metric_events (event_id) VALUES ($1) \
                 ON CONFLICT (event_id) DO NOTHING",
                [event.event_id.clone().into()],
            ))
            .await
            .map_err(|e| AppError::Database(format!("Dedup insert: {e}")))?;

        if dedup.rows_affected() == 0 {
            txn.commit().await
                .map_err(|e| AppError::Database(format!("Commit: {e}")))?;
            tracing::debug!(event_id = %event.event_id, "Duplicate event — idempotent skip");
            return Ok(false);
        }

        // Step 2 — Upsert endpoint_metrics.
        //
        // avg_latency uses a weighted running-average formula so it stays
        // correct as events accumulate in the same time bucket:
        //   new_avg = (old_avg × old_n + new_value) / (old_n + 1)
        txn.execute(Statement::from_sql_and_values(
            DbBackend::Postgres,
            UPSERT_ENDPOINT_METRICS,
            [
                event.client_id.clone().into(),
                event.service_name.clone().into(),
                event.endpoint.clone().into(),
                event.method.clone().into(),
                error_inc.into(),
                event.latency_ms.into(),
                // Histogram (23 values)
                histogram.le_5ms.into(),     histogram.le_10ms.into(),
                histogram.le_25ms.into(),    histogram.le_50ms.into(),
                histogram.le_75ms.into(),    histogram.le_100ms.into(),
                histogram.le_150ms.into(),   histogram.le_200ms.into(),
                histogram.le_300ms.into(),   histogram.le_400ms.into(),
                histogram.le_500ms.into(),   histogram.le_750ms.into(),
                histogram.le_1000ms.into(),  histogram.le_1500ms.into(),
                histogram.le_2000ms.into(),  histogram.le_3000ms.into(),
                histogram.le_5000ms.into(),  histogram.le_7500ms.into(),
                histogram.le_10000ms.into(), histogram.le_15000ms.into(),
                histogram.le_20000ms.into(), histogram.le_30000ms.into(),
                histogram.le_60000ms.into(),
                // Status (5 values)
                inc_1xx.into(), inc_2xx.into(), inc_3xx.into(),
                inc_4xx.into(), inc_5xx.into(),
                // Time bucket
                time_bucket.into(),
            ],
        ))
        .await
        .map_err(|e| AppError::Database(format!("Metrics upsert: {e}")))?;

        // Commit — dedup record and metrics update land atomically.
        txn.commit().await
            .map_err(|e| AppError::Database(format!("Commit: {e}")))?;

        tracing::debug!(
            event_id    = %event.event_id,
            client_id   = %event.client_id,
            endpoint    = %event.endpoint,
            latency_ms  = event.latency_ms,
            status_code = event.status_code,
            "Metrics event committed"
        );
        Ok(true)
    }

    async fn delete_expired_dedup_records(
        &self,
        retain_until: DateTime<Utc>,
    ) -> Result<u64, AppError> {
        let result = self.db
            .execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "DELETE FROM processed_metric_events WHERE processed_at < $1",
                [retain_until.into()],
            ))
            .await
            .map_err(|e| AppError::Database(format!("Dedup cleanup: {e}")))?;

        let deleted = result.rows_affected();
        tracing::info!(deleted, "Expired dedup records removed");
        Ok(deleted)
    }

    async fn get_overall_stats(
        &self,
        client_id: Option<&str>,
        start_time: DateTime<Utc>,
        end_time:   DateTime<Utc>,
    ) -> Result<OverallStats, AppError> {
        let (sql, values) = if let Some(cid) = client_id {
            (
                r#"SELECT
                    COALESCE(SUM(total_hits), 0)                                               AS total_hits,
                    COALESCE(SUM(error_hits), 0)                                               AS error_hits,
                    COALESCE(SUM(avg_latency * total_hits) / NULLIF(SUM(total_hits), 0), 0.0) AS avg_latency,
                    COUNT(DISTINCT service_name)                                                AS unique_services,
                    COUNT(DISTINCT endpoint || '|' || method)                                   AS unique_endpoints
                FROM endpoint_metrics
                WHERE client_id = $1 AND time_bucket >= $2 AND time_bucket <= $3"#,
                vec![cid.into(), truncate_to_hour(start_time).naive_utc().into(), end_time.naive_utc().into()],
            )
        } else {
            (
                r#"SELECT
                    COALESCE(SUM(total_hits), 0)                                               AS total_hits,
                    COALESCE(SUM(error_hits), 0)                                               AS error_hits,
                    COALESCE(SUM(avg_latency * total_hits) / NULLIF(SUM(total_hits), 0), 0.0) AS avg_latency,
                    COUNT(DISTINCT service_name)                                                AS unique_services,
                    COUNT(DISTINCT endpoint || '|' || method)                                   AS unique_endpoints
                FROM endpoint_metrics
                WHERE time_bucket >= $1 AND time_bucket <= $2"#,
                vec![truncate_to_hour(start_time).naive_utc().into(), end_time.naive_utc().into()],
            )
        };

        let row = OverallStatsRow::find_by_statement(
            Statement::from_sql_and_values(DbBackend::Postgres, sql, values),
        )
        .one(&self.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::Database("Stats query returned no row".into()))?;

        Ok(OverallStats {
            total_hits:       row.total_hits,
            error_hits:       row.error_hits,
            avg_latency:      row.avg_latency,
            unique_services:  row.unique_services,
            unique_endpoints: row.unique_endpoints,
        })
    }

    async fn get_top_endpoints(
        &self,
        client_id:  Option<&str>,
        limit:      i64,
        start_time: Option<DateTime<Utc>>,
        end_time:   Option<DateTime<Utc>>,
    ) -> Result<Vec<EndpointStat>, AppError> {
        let limit = limit.clamp(1, MAX_QUERY_LIMIT);

        // Build WHERE clause dynamically to avoid 4-way code duplication.
        let mut conditions = String::from("WHERE TRUE");
        let mut values: Vec<sea_orm::Value> = vec![];
        let mut param = 1usize;

        if let Some(cid) = client_id {
            conditions.push_str(&format!(" AND client_id = ${param}"));
            values.push(cid.into());
            param += 1;
        }
        if let Some(st) = start_time {
            conditions.push_str(&format!(" AND time_bucket >= ${param}"));
            values.push(truncate_to_hour(st).naive_utc().into());
            param += 1;
        }
        if let Some(et) = end_time {
            conditions.push_str(&format!(" AND time_bucket <= ${param}"));
            values.push(et.naive_utc().into());
            param += 1;
        }
        let sql = format!(
            r#"SELECT service_name, endpoint, method,
                SUM(total_hits)                                                AS total_hits,
                SUM(avg_latency * total_hits) / NULLIF(SUM(total_hits), 0)    AS avg_latency,
                SUM(error_hits)                                                AS error_hits
            FROM endpoint_metrics
            {conditions}
            GROUP BY service_name, endpoint, method
            ORDER BY total_hits DESC LIMIT ${param}"#
        );
        values.push(limit.into());

        let rows = EndpointStatRow::find_by_statement(
            Statement::from_sql_and_values(DbBackend::Postgres, &sql, values),
        )
        .all(&self.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| EndpointStat {
            service_name: r.service_name,
            endpoint:     r.endpoint,
            method:       r.method,
            total_hits:   r.total_hits,
            avg_latency:  r.avg_latency,
            error_hits:   r.error_hits,
        }).collect())
    }

    async fn get_time_series(
        &self,
        client_id:  Option<&str>,
        start_time: DateTime<Utc>,
        end_time:   DateTime<Utc>,
        limit:      i64,
    ) -> Result<Vec<TimeSeriesEntry>, AppError> {
        let limit = limit.clamp(1, MAX_QUERY_LIMIT);

        let (sql, values) = if let Some(cid) = client_id {
            (
                r#"SELECT service_name, endpoint, method,
                    SUM(total_hits)                                                AS total_hits,
                    SUM(error_hits)                                                AS error_hits,
                    SUM(avg_latency * total_hits) / NULLIF(SUM(total_hits), 0)    AS avg_latency,
                    MIN(min_latency)                                               AS min_latency,
                    MAX(max_latency)                                               AS max_latency,
                    time_bucket
                FROM endpoint_metrics
                WHERE client_id = $1 AND time_bucket >= $2 AND time_bucket <= $3
                GROUP BY service_name, endpoint, method, time_bucket
                ORDER BY time_bucket DESC LIMIT $4"#,
                vec![cid.into(), start_time.naive_utc().into(), end_time.naive_utc().into(), limit.into()],
            )
        } else {
            (
                r#"SELECT service_name, endpoint, method,
                    SUM(total_hits)                                                AS total_hits,
                    SUM(error_hits)                                                AS error_hits,
                    SUM(avg_latency * total_hits) / NULLIF(SUM(total_hits), 0)    AS avg_latency,
                    MIN(min_latency)                                               AS min_latency,
                    MAX(max_latency)                                               AS max_latency,
                    time_bucket
                FROM endpoint_metrics
                WHERE time_bucket >= $1 AND time_bucket <= $2
                GROUP BY service_name, endpoint, method, time_bucket
                ORDER BY time_bucket DESC LIMIT $3"#,
                vec![start_time.naive_utc().into(), end_time.naive_utc().into(), limit.into()],
            )
        };

        let rows = TimeSeriesRow::find_by_statement(
            Statement::from_sql_and_values(DbBackend::Postgres, sql, values),
        )
        .all(&self.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| TimeSeriesEntry {
            service_name: r.service_name,
            endpoint:     r.endpoint,
            method:       r.method,
            total_hits:   r.total_hits,
            error_hits:   r.error_hits,
            avg_latency:  r.avg_latency,
            min_latency:  r.min_latency,
            max_latency:  r.max_latency,
            time_bucket:  r.time_bucket.and_utc(),
        }).collect())
    }

    async fn get_endpoint_metrics_page(
        &self,
        client_id:     &str,
        tenant_config: &TenantConfig,
        limit:         i64,
        offset:        i64,
    ) -> Result<(Vec<ApiMetricsEntry>, i64), AppError> {
        // F7: run COUNT and data queries in parallel — halves wall-clock time.
        //
        // ⚠️  KNOWN LIMITATION (issue #7): The COUNT query has NO time-range filter.
        // This is intentional — the paginated `/api/analytics/apis` endpoint currently
        // does NOT accept start_time/end_time, so the count is correct for the current
        // API contract.
        //
        // FUTURE TRAP: if a time-range parameter is ever added to the paginated endpoint
        // list, this COUNT query MUST also receive the same filter, or the total_count in
        // pagination metadata will be wrong (it will over-count endpoints that had traffic
        // before the window but not within it). Both the count_sql and data_sql must be
        // updated together.
        let count_sql = r#"
            SELECT COUNT(*) AS count FROM (
                SELECT 1 FROM endpoint_metrics WHERE client_id = $1
                GROUP BY service_name, endpoint, method
            ) t
        "#;
        let data_sql = format!(
            r#"SELECT service_name, endpoint, method,
                SUM(total_hits)                                                AS total_hits,
                SUM(error_hits)                                                AS error_hits,
                SUM(avg_latency * total_hits) / NULLIF(SUM(total_hits), 0)    AS avg_latency,
                MIN(min_latency)                                               AS min_latency,
                MAX(max_latency)                                               AS max_latency,
                COUNT(*)                                                       AS active_buckets,
                {}
            FROM endpoint_metrics
            WHERE client_id = $1
            GROUP BY service_name, endpoint, method
            ORDER BY total_hits DESC, service_name, endpoint
            LIMIT $2 OFFSET $3"#,
            HISTOGRAM_SELECT
        );

        let (count_result, rows_result) = tokio::try_join!(
            async {
                CountRow::find_by_statement(Statement::from_sql_and_values(
                    DbBackend::Postgres, count_sql, [client_id.into()],
                ))
                .one(&self.db)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?
                .ok_or_else(|| AppError::Database("Count query returned no row".into()))
                .map(|r| r.count)
            },
            async {
                EndpointMetricsRow::find_by_statement(Statement::from_sql_and_values(
                    DbBackend::Postgres, &data_sql,
                    [client_id.into(), limit.into(), offset.into()],
                ))
                .all(&self.db)
                .await
                .map_err(|e| AppError::Database(e.to_string()))
            },
        )?;

        let total = count_result;

        let items = rows_result.into_iter().map(|r| {
            // ⚠️  APPROXIMATION (issue #11): `active_buckets` is COUNT(*) of hourly
            // time-bucket rows — not a bounded time window. For endpoints with sparse
            // traffic (e.g. one hit at 9 AM, one at 5 PM) this returns 2 active hours
            // but the actual elapsed window is 8 hours, producing throughput_rpm that
            // is 4× too high. This approximation is acceptable for the all-time
            // paginated list where no explicit window is available. The `/percentiles`
            // endpoint uses the exact (end_time − start_time) window, which is correct.
            let active_hours   = r.active_buckets.unwrap_or(1).max(1) as f64;
            let window_minutes = active_hours * 60.0;
            let metrics = build_endpoint_metrics(&r, tenant_config, window_minutes);
            ApiMetricsEntry {
                service_name: r.service_name,
                endpoint:     r.endpoint,
                method:       r.method,
                total_hits:   r.total_hits,
                error_hits:   r.error_hits,
                avg_latency:  r.avg_latency,
                min_latency:  r.min_latency,
                max_latency:  r.max_latency,
                metrics,
            }
        }).collect();

        Ok((items, total))
    }


    async fn get_endpoint_metrics(
        &self,
        client_id:     &str,
        service_name:  &str,
        endpoint:      &str,
        method:        &str,
        tenant_config: &TenantConfig,
        start_time:    DateTime<Utc>,
        end_time:      DateTime<Utc>,
    ) -> Result<EndpointMetrics, AppError> {
        let sql = format!(
            r#"SELECT service_name, endpoint, method,
                SUM(total_hits)                                                AS total_hits,
                SUM(error_hits)                                                AS error_hits,
                SUM(avg_latency * total_hits) / NULLIF(SUM(total_hits), 0)    AS avg_latency,
                MIN(min_latency)                                               AS min_latency,
                MAX(max_latency)                                               AS max_latency,
                COUNT(*)                                                       AS active_buckets,
                {}
            FROM endpoint_metrics
            WHERE client_id = $1 AND service_name = $2
              AND endpoint   = $3 AND method       = $4
              AND time_bucket >= $5 AND time_bucket <= $6
            GROUP BY service_name, endpoint, method"#,
            HISTOGRAM_SELECT
        );

        let maybe_row = EndpointMetricsRow::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres, &sql,
            [
                client_id.into(), service_name.into(), endpoint.into(),
                method.into(), truncate_to_hour(start_time).naive_utc().into(),
                end_time.naive_utc().into(),
            ],
        ))
        .one(&self.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let row = match maybe_row {
            Some(r) => r,
            None    => return Ok(EndpointMetrics::default()),
        };

        let window_minutes = ((end_time - start_time).num_seconds() as f64 / 60.0).max(1.0);
        Ok(build_endpoint_metrics(&row, tenant_config, window_minutes))
    }

    async fn delete_client_metrics_before(
        &self,
        client_id:    &str,
        retain_until: DateTime<Utc>,
    ) -> Result<u64, AppError> {
        let result = self.db
            .execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "DELETE FROM endpoint_metrics WHERE client_id = $1 AND time_bucket < $2",
                [client_id.into(), retain_until.into()],
            ))
            .await
            .map_err(|e| AppError::Database(format!("Retention cleanup: {e}")))?;

        Ok(result.rows_affected())
    }

    // ── Service-level analytics ───────────────────────────────────────────────

    async fn get_all_services_summary(
        &self,
        client_id:  &str,
        start_time: DateTime<Utc>,
        end_time:   DateTime<Utc>,
    ) -> Result<Vec<ServiceSummary>, AppError> {
        let sql = format!(
            r#"SELECT
                service_name,
                SUM(total_hits)                                                          AS total_hits,
                SUM(error_hits)                                                          AS error_hits,
                SUM(avg_latency * total_hits) / NULLIF(SUM(total_hits), 0)              AS avg_latency,
                MIN(min_latency)                                                         AS min_latency,
                MAX(max_latency)                                                         AS max_latency,
                COUNT(DISTINCT endpoint || '|' || method)                                AS active_buckets,
                COUNT(*)                                                                 AS endpoint_count,
                {}
            FROM endpoint_metrics
            WHERE client_id = $1
              AND time_bucket >= $2
              AND time_bucket <= $3
            GROUP BY service_name
            ORDER BY total_hits DESC"#,
            HISTOGRAM_SELECT
        );

        let rows = EndpointMetricsRow::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres,
            &sql,
            [
                client_id.into(),
                truncate_to_hour(start_time).naive_utc().into(),
                end_time.naive_utc().into(),
            ],
        ))
        .all(&self.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| ServiceSummary {
            service_name:   r.service_name.clone(),
            total_hits:     r.total_hits,
            error_hits:     r.error_hits,
            avg_latency:    r.avg_latency,
            min_latency:    r.min_latency,
            max_latency:    r.max_latency,
            // active_buckets holds COUNT(DISTINCT endpoint||method) due to alias reuse
            endpoint_count: r.active_buckets.unwrap_or(0),
            active_buckets: 1, // single time range — not used for throughput here
            histogram_counts: bucket_counts(&r),
            status_distribution: crate::domain::metrics::StatusDistribution {
                status_1xx: r.status_1xx,
                status_2xx: r.status_2xx,
                status_3xx: r.status_3xx,
                status_4xx: r.status_4xx,
                status_5xx: r.status_5xx,
            },
        }).collect())
    }

    async fn get_service_summary(
        &self,
        client_id:    &str,
        service_name: &str,
        start_time:   DateTime<Utc>,
        end_time:     DateTime<Utc>,
    ) -> Result<Option<ServiceSummary>, AppError> {
        let sql = format!(
            r#"SELECT
                service_name,
                SUM(total_hits)                                                          AS total_hits,
                SUM(error_hits)                                                          AS error_hits,
                SUM(avg_latency * total_hits) / NULLIF(SUM(total_hits), 0)              AS avg_latency,
                MIN(min_latency)                                                         AS min_latency,
                MAX(max_latency)                                                         AS max_latency,
                COUNT(DISTINCT endpoint || '|' || method)                                AS active_buckets,
                COUNT(*)                                                                 AS endpoint_count,
                {}
            FROM endpoint_metrics
            WHERE client_id = $1
              AND service_name = $2
              AND time_bucket >= $3
              AND time_bucket <= $4
            GROUP BY service_name"#,
            HISTOGRAM_SELECT
        );

        let maybe_row = EndpointMetricsRow::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres,
            &sql,
            [
                client_id.into(),
                service_name.into(),
                truncate_to_hour(start_time).naive_utc().into(),
                end_time.naive_utc().into(),
            ],
        ))
        .one(&self.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(maybe_row.map(|r| {
            let window_minutes = ((end_time - start_time).num_seconds() as f64 / 60.0).max(1.0);
            ServiceSummary {
                service_name:   r.service_name.clone(),
                total_hits:     r.total_hits,
                error_hits:     r.error_hits,
                avg_latency:    r.avg_latency,
                min_latency:    r.min_latency,
                max_latency:    r.max_latency,
                endpoint_count: r.active_buckets.unwrap_or(0),
                active_buckets: (window_minutes / 60.0).ceil() as i64,
                histogram_counts: bucket_counts(&r),
                status_distribution: crate::domain::metrics::StatusDistribution {
                    status_1xx: r.status_1xx,
                    status_2xx: r.status_2xx,
                    status_3xx: r.status_3xx,
                    status_4xx: r.status_4xx,
                    status_5xx: r.status_5xx,
                },
            }
        }))
    }

    async fn get_service_endpoints(
        &self,
        client_id:     &str,
        service_name:  &str,
        tenant_config: &TenantConfig,
        start_time:    DateTime<Utc>,
        end_time:      DateTime<Utc>,
    ) -> Result<Vec<crate::domain::metrics::ServiceEndpointStat>, AppError> {
        let sql = format!(
            r#"SELECT service_name, endpoint, method,
                SUM(total_hits)                                                          AS total_hits,
                SUM(error_hits)                                                          AS error_hits,
                SUM(avg_latency * total_hits) / NULLIF(SUM(total_hits), 0)              AS avg_latency,
                MIN(min_latency)                                                         AS min_latency,
                MAX(max_latency)                                                         AS max_latency,
                COUNT(*)                                                                 AS active_buckets,
                {}
            FROM endpoint_metrics
            WHERE client_id = $1
              AND service_name = $2
              AND time_bucket >= $3
              AND time_bucket <= $4
            GROUP BY service_name, endpoint, method
            ORDER BY total_hits DESC"#,
            HISTOGRAM_SELECT
        );

        let rows = EndpointMetricsRow::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres,
            &sql,
            [
                client_id.into(),
                service_name.into(),
                truncate_to_hour(start_time).naive_utc().into(),
                end_time.naive_utc().into(),
            ],
        ))
        .all(&self.db)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let _window_minutes = ((end_time - start_time).num_seconds() as f64 / 60.0).max(1.0);

        Ok(rows.into_iter().map(|r| {
            let counts = bucket_counts(&r);
            let apdex  = tenant_config.compute_apdex(&counts, r.total_hits);
            let p99    = tenant_config.compute_percentiles(&counts, r.total_hits).p99;
            let error_rate = if r.total_hits > 0 {
                (r.error_hits as f64 / r.total_hits as f64) * 100.0
            } else {
                0.0
            };
            crate::domain::metrics::ServiceEndpointStat {
                endpoint:    r.endpoint,
                method:      r.method,
                total_hits:  r.total_hits,
                error_hits:  r.error_hits,
                avg_latency: r.avg_latency,
                p99_latency: p99,
                apdex_score: apdex.score,
                error_rate,
            }
        }).collect())
    }
}

