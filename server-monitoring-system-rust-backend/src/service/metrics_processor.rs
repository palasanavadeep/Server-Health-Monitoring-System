use std::sync::Arc;

use crate::domain::ingest::MetricsEvent;
use crate::error::app_error::AppError;
use crate::repository::metrics_repo::MetricsRepository;

/// Metrics processor service — PostgreSQL analytics aggregation.
///
/// ## Single responsibility
///
/// This service knows only about PostgreSQL. It has no knowledge of MongoDB,
/// RabbitMQ channels, or the upstream persistence pipeline.
///
/// ## Idempotency
///
/// Idempotency is enforced at the repository level via an atomic PostgreSQL
/// transaction (`processed_metric_events` dedup + `endpoint_metrics` upsert).
/// This service simply delegates — it contains no SQL or transaction logic.
///
/// ## Dedup retention
///
/// Processed event IDs are stored for 30 days (configurable via cleanup job).
/// Call `run_cleanup()` periodically to prevent unbounded table growth.
pub struct MetricsProcessorService {
    metrics_repository: Arc<dyn MetricsRepository>,
}

impl MetricsProcessorService {
    pub fn new(metrics_repository: Arc<dyn MetricsRepository>) -> Self {
        Self { metrics_repository }
    }

    /// Process a metrics event idempotently.
    ///
    /// Delegates to the repository's atomic transaction which:
    ///   1. Inserts `event_id` into `processed_metric_events` (ON CONFLICT DO NOTHING)
    ///   2. If new: upserts `endpoint_metrics`
    ///   3. Commits both atomically
    ///
    /// # Returns
    /// - `Ok(true)`  — event was new; metrics updated
    /// - `Ok(false)` — duplicate `event_id`; safely skipped, no double-count
    /// - `Err`       — database failure; caller should NACK and retry
    #[tracing::instrument(skip(self), fields(event_id = %event.event_id, client_id = %event.client_id))]
    pub async fn process_metrics(&self, event: MetricsEvent) -> Result<bool, AppError> {
        let result = self
            .metrics_repository
            .process_event_idempotently(&event)
            .await?;

        if result {
            tracing::info!("Metrics event processed");
        } else {
            tracing::debug!("Duplicate metrics event — skipped");
        }

        Ok(result)
    }

    /// Delete deduplication records older than `retain_days` days.
    ///
    /// Should be called on a daily schedule. The default retention is 30 days —
    /// this must exceed the maximum RabbitMQ message replay window.
    pub async fn run_cleanup(&self, retain_days: i64) -> Result<u64, AppError> {
        let retain_until = chrono::Utc::now() - chrono::Duration::days(retain_days);
        self.metrics_repository
            .cleanup_processed_events(retain_until)
            .await
    }
}
