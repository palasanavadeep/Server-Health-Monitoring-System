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

        // Extract fields we need after the move into ApiHit
        let event_id = data.event_id.clone();
        let client_id = data.client_id.clone();
        let service_name = data.service_name.clone();
        let endpoint = data.endpoint.clone();
        let method = data.method.clone();
        let status_code = data.status_code;
        let latency_ms = data.latency_ms;
        let timestamp = data.timestamp;

        let hit = ApiHit {
            id: None,
            event_id: data.event_id,
            client_id: data.client_id,
            api_key_id: data.api_key_id,
            service_name: data.service_name,
            endpoint: data.endpoint,
            method: data.method,
            status_code: data.status_code as i32,
            latency_ms: data.latency_ms,
            ip: if data.ip.is_empty() {
                None
            } else {
                Some(data.ip)
            },
            user_agent: if data.user_agent.is_empty() {
                None
            } else {
                Some(data.user_agent)
            },
            timestamp: Some(data.timestamp),
            created_at: Some(Utc::now()),
        };

        // STEP 1: Save raw event to MongoDB
        if let Err(e) = self.api_hit_repository.save(&hit).await {
            tracing::error!(
                event_id = %event_id,
                error = %e,
                "Critical: failed to save raw event to MongoDB"
            );
            return Err(e);
        }
        tracing::info!(event_id = %event_id, "Raw event saved to MongoDB");

        // STEP 2: Upsert aggregated metrics in PostgreSQL
        let time_bucket = Self::time_bucket(timestamp);
        let error_hits: i32 = if status_code >= 400 { 1 } else { 0 };

        if let Err(e) = self
            .metrics_repository
            .upsert_endpoint_metrics(
                &client_id,
                &service_name,
                &endpoint,
                &method,
                1,
                error_hits,
                latency_ms,
                latency_ms,
                latency_ms,
                time_bucket,
            )
            .await
        {
            // Non-fatal: raw event is already saved
            tracing::error!(
                event_id = %event_id,
                error = %e,
                "Non-critical: raw event saved but metrics update failed"
            );
        } else {
            tracing::info!(event_id = %event_id, "Event processed successfully");
        }

        Ok(())
    }

    /// Delete raw events older than `days_to_keep` days.
    pub async fn cleanup_old_events(&self, days_to_keep: i64) -> Result<u64, AppError> {
        let cutoff = Utc::now() - chrono::Duration::days(days_to_keep);
        self.api_hit_repository.delete_old_hits(cutoff).await
    }
}
