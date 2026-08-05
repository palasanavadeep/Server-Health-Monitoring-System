use chrono::{Utc, DateTime, Timelike};

use crate::errors::app_error::AppError;
use super::api_hit_repository::ApiHitRepository;
use super::metrics_repository::MetricsRepository;

/// ProcessorService - Processes consumed events.
/// Mirrors Node.js ProcessorService class.
pub struct ProcessorService {
    api_hit_repository: ApiHitRepository,
    metrics_repository: MetricsRepository,
}

impl ProcessorService {
    pub fn new(
        api_hit_repository: ApiHitRepository,
        metrics_repository: MetricsRepository,
    ) -> Self {
        Self {
            api_hit_repository,
            metrics_repository,
        }
    }

    /// Get time bucket (truncate to hour).
    /// Mirrors Node.js getTimeBucket().
    fn get_time_bucket(timestamp: &str) -> DateTime<Utc> {
        let dt = timestamp
            .parse::<DateTime<Utc>>()
            .unwrap_or_else(|_| Utc::now());

        // Truncate to the start of the hour
        dt.with_minute(0)
            .and_then(|d| d.with_second(0))
            .and_then(|d| d.with_nanosecond(0))
            .unwrap_or(dt)
    }

    /// Process a single event.
    /// Mirrors Node.js processEvent(eventData).
    pub async fn process_event(&self, event_data: serde_json::Value) -> Result<(), AppError> {
        let event_id = event_data
            .get("eventId")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        tracing::info!(
            event_id = %event_id,
            client_id = event_data.get("clientId").and_then(|v| v.as_str()).unwrap_or(""),
            service_name = event_data.get("serviceName").and_then(|v| v.as_str()).unwrap_or(""),
            endpoint = event_data.get("endpoint").and_then(|v| v.as_str()).unwrap_or(""),
            method = event_data.get("method").and_then(|v| v.as_str()).unwrap_or(""),
            "Processing event data"
        );

        // STEP 1: Save raw event to MongoDB
        // raw_saved tracks if MongoDB write succeeded, used in metrics fallback below
        let raw_saved;
        match self.api_hit_repository.save(event_data.clone()).await {
            Ok(_) => {
                raw_saved = true;
                tracing::info!(event_id = %event_id, "Raw event saved to MongoDB");
            }
            Err(e) => {
                tracing::error!(
                    event_id = %event_id,
                    error = %e,
                    "Critical: Failed to save raw event to MongoDB"
                );
                return Err(e);
            }
        }

        // STEP 2: Update aggregated metrics in PostgreSQL
        if let Err(e) = self.update_metrics_with_fallback(&event_data).await {
            if !raw_saved {
                return Err(e);
            }
            tracing::error!(
                event_id = %event_id,
                error = %e,
                "Non-critical: Raw event saved but metrics update failed"
            );
        } else {
            tracing::info!(event_id = %event_id, "Event processed successfully");
        }

        Ok(())
    }

    async fn update_metrics_with_fallback(
        &self,
        event_data: &serde_json::Value,
    ) -> Result<(), AppError> {
        let timestamp = event_data
            .get("timestamp")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let time_bucket = Self::get_time_bucket(timestamp);

        let client_id = event_data.get("clientId").and_then(|v| v.as_str()).unwrap_or("");
        let service_name = event_data.get("serviceName").and_then(|v| v.as_str()).unwrap_or("");
        let endpoint = event_data.get("endpoint").and_then(|v| v.as_str()).unwrap_or("");
        let method = event_data.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let status_code = event_data.get("statusCode").and_then(|v| v.as_i64()).unwrap_or(0);
        let latency_ms = event_data.get("latencyMs").and_then(|v| v.as_f64()).unwrap_or(0.0);

        let error_hits = if status_code >= 400 { 1 } else { 0 };

        self.metrics_repository
            .upsert_endpoint_metrics(
                client_id,
                service_name,
                endpoint,
                method,
                1, // totalHits
                error_hits,
                latency_ms,
                latency_ms,
                latency_ms,
                time_bucket,
            )
            .await?;

        tracing::info!(
            event_id = event_data.get("eventId").and_then(|v| v.as_str()).unwrap_or(""),
            "Metrics updated successfully"
        );

        Ok(())
    }

    /// Cleanup old events.
    pub async fn cleanup_old_events(&self, days_to_keep: i64) -> Result<u64, AppError> {
        let cutoff = Utc::now() - chrono::Duration::days(days_to_keep);
        self.api_hit_repository.delete_old_hits(cutoff).await
    }

    /// Get a reference to the metrics repository (used by analytics service).
    pub fn metrics_repository(&self) -> &MetricsRepository {
        &self.metrics_repository
    }
}
