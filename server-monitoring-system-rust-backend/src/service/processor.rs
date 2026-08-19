use chrono::{DateTime, Timelike, Utc};
use std::sync::Arc;

use crate::domain::api_hit::ApiHit;
use crate::domain::ingest::HitEventData;
use crate::error::app_error::AppError;
use crate::repository::api_hit_repo::ApiHitRepository;
use crate::repository::metrics_repo::MetricsRepository;

/// Processor service — handles the dual-write pattern.
///
/// Operates on typed `HitEventData` structs from the consumer — no `serde_json::Value`
/// field lookups on the hot consumer path.
pub struct ProcessorService {
    api_hit_repository: Arc<dyn ApiHitRepository>,
    metrics_repository: Arc<dyn MetricsRepository>,
}

impl ProcessorService {
    pub fn new(
        api_hit_repository: Arc<dyn ApiHitRepository>,
        metrics_repository: Arc<dyn MetricsRepository>,
    ) -> Self {
        Self {
            api_hit_repository,
            metrics_repository,
        }
    }

    /// Truncate a timestamp to the start of the hour (time-bucket for metrics).
    fn time_bucket(dt: DateTime<Utc>) -> DateTime<Utc> {
        dt.with_minute(0)
            .and_then(|d| d.with_second(0))
            .and_then(|d| d.with_nanosecond(0))
            .unwrap_or(dt)
    }

    /// Process a single event — save raw data to MongoDB and update PG metrics.
    pub async fn process_event(&self, data: HitEventData) -> Result<(), AppError> {
        tracing::info!(
            event_id = %data.event_id,
            client_id = %data.client_id,
            service_name = %data.service_name,
            endpoint = %data.endpoint,
            method = %data.method,
            "Processing event"
        );

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
            ip: Some(data.ip.clone()),
            user_agent: if data.user_agent.is_empty() {
                None
            } else {
                Some(data.user_agent.clone())
            },
            timestamp: Some(data.timestamp),
            created_at: Some(Utc::now()),
        };

        // STEP 1: Save raw event to MongoDB
        match self.api_hit_repository.save(&hit).await {
            Ok(_) => {
                tracing::info!(event_id = %data.event_id, "Raw event saved to MongoDB");
            }
            Err(e) => {
                tracing::error!(
                    event_id = %data.event_id,
                    error = %e,
                    "Critical: failed to save raw event to MongoDB"
                );
                return Err(e);
            }
        }

        // STEP 2: Upsert aggregated metrics in PostgreSQL
        let time_bucket = Self::time_bucket(data.timestamp);
        let error_hits: i32 = if data.status_code >= 400 { 1 } else { 0 };

        if let Err(e) = self
            .metrics_repository
            .upsert_endpoint_metrics(
                &data.client_id,
                &data.service_name,
                &data.endpoint,
                &data.method,
                1,
                error_hits,
                data.latency_ms,
                data.latency_ms,
                data.latency_ms,
                time_bucket,
            )
            .await
        {
            // Non-fatal: raw event is already saved
            tracing::error!(
                event_id = %data.event_id,
                error = %e,
                "Non-critical: raw event saved but metrics update failed"
            );
        } else {
            tracing::info!(event_id = %data.event_id, "Event processed successfully");
        }

        Ok(())
    }

    /// Delete raw events older than `days_to_keep` days.
    pub async fn cleanup_old_events(&self, days_to_keep: i64) -> Result<u64, AppError> {
        let cutoff = Utc::now() - chrono::Duration::days(days_to_keep);
        self.api_hit_repository.delete_old_hits(cutoff).await
    }
}
