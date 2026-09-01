use chrono::Utc;
use std::sync::Arc;

use crate::domain::api_hit::ApiHit;
use crate::domain::ingest::{HitEventData, MetricsEvent};
use crate::error::app_error::AppError;
use crate::repository::api_hit_repo::ApiHitRepository;

/// Persistence service — writes raw API hit events to MongoDB.
///
/// ## Single responsibility
///
/// This service knows only about MongoDB. It has no knowledge of PostgreSQL,
/// RabbitMQ, or the downstream metrics pipeline. The consumer owns that
/// orchestration.
///
/// ## Idempotency
///
/// `persist()` is idempotent: if the underlying `ApiHitRepository` detects a
/// duplicate `event_id`, it returns `Ok(false)` and the service still produces
/// a `MetricsEvent`. This allows the persistence consumer to safely retry and
/// continue publishing to the metrics queue without double-storing raw events.
pub struct PersistenceService {
    api_hit_repository: Arc<dyn ApiHitRepository>,
}

impl PersistenceService {
    pub fn new(api_hit_repository: Arc<dyn ApiHitRepository>) -> Self {
        Self { api_hit_repository }
    }

    /// Persist a raw API hit to MongoDB and return the downstream `MetricsEvent`.
    ///
    /// On success (new or duplicate): returns `Ok(MetricsEvent)`.
    /// On genuine infrastructure error: returns `Err` — caller should NACK.
    ///
    /// The `MetricsEvent.timestamp` is taken from the original event, not `Utc::now()`,
    /// ensuring time-bucket aggregation is always correct regardless of processing delay.
    #[tracing::instrument(skip(self), fields(event_id = %data.event_id, client_id = %data.client_id))]
    pub async fn persist(&self, data: HitEventData) -> Result<MetricsEvent, AppError> {
        let hit = ApiHit {
            id: None,
            event_id: data.event_id.clone(),
            client_id: data.client_id.clone(),
            api_key_id: data.api_key_id.clone(),
            service_name: data.service_name.clone(),
            endpoint: data.endpoint.clone(),
            method: data.method.clone(),
            status_code: data.status_code as i32,
            latency_ms: data.latency_ms,
            ip: if data.ip.is_empty() { None } else { Some(data.ip.clone()) },
            user_agent: if data.user_agent.is_empty() {
                None
            } else {
                Some(data.user_agent.clone())
            },
            timestamp: Some(data.timestamp),
            created_at: Some(Utc::now()),
        };

        // save() is idempotent — duplicate returns Ok(false), not Err.
        let inserted = self.api_hit_repository.save(&hit).await?;
        if inserted {
            tracing::info!("Raw event persisted to MongoDB");
        } else {
            tracing::debug!("Duplicate event_id in MongoDB — continuing to metrics publish");
        }

        // Return the metrics-relevant subset. Always uses original event timestamp.
        Ok(MetricsEvent {
            event_id: data.event_id,
            client_id: data.client_id,
            service_name: data.service_name,
            endpoint: data.endpoint,
            method: data.method,
            status_code: data.status_code,
            latency_ms: data.latency_ms,
            timestamp: data.timestamp, // original event time — never Utc::now()
        })
    }

    /// Delete raw events older than `days_to_keep` days.
    pub async fn cleanup_old_events(&self, days_to_keep: i64) -> Result<u64, AppError> {
        let cutoff = Utc::now() - chrono::Duration::days(days_to_keep);
        self.api_hit_repository.delete_old_hits(cutoff).await
    }
}
