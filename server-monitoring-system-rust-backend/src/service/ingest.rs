use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::cache::ingest_quota_tracker::{IngestQuotaTracker, QuotaCheckResult};
use crate::domain::ingest::{HitEvent, HitEventData};
use crate::domain::tenant_config::TenantConfig;
use crate::dto::request::ingest::IngestHitRequest;
use crate::dto::response::ingest::{
    BatchEventError, BatchIngestResponse, IngestResponse, IngestStatus,
};
use crate::error::app_error::AppError;
use crate::messaging::producer::EventProducer;

// ── Ingest service ────────────────────────────────────────────────────────────

/// Ingest service — validates, quota-checks, and publishes API hit events.
///
/// Accepts strongly typed request structs rather than raw JSON to eliminate
/// runtime field-lookup overhead on the hot ingest path.
pub struct IngestService {
    producer:      EventProducer,
    quota_tracker: Arc<IngestQuotaTracker>,
}

impl IngestService {
    pub fn new(
        producer:      EventProducer,
        quota_tracker: Arc<IngestQuotaTracker>,
    ) -> Self {
        Self { producer, quota_tracker }
    }

    // ── Single-event ingest ───────────────────────────────────────────────────

    /// Ingest a single API hit event.
    ///
    /// Validates fields, checks the tenant's daily quota, enriches with a
    /// UUID event_id and timestamp, then publishes to RabbitMQ.
    #[tracing::instrument(
        skip(self, tenant_config),
        fields(service_name = %req.service_name, endpoint = %req.endpoint)
    )]
    pub async fn ingest_hit(
        &self,
        req:           IngestHitRequest,
        tenant_config: &TenantConfig,
    ) -> Result<IngestResponse, AppError> {
        self.validate(&req)?;

        // Quota check — single event contributes 1 unit.
        if let QuotaCheckResult::Exceeded { count, limit } = self
            .quota_tracker
            .check_and_increment(&req.client_id, tenant_config.daily_ingest_quota, 1)
        {
            tracing::warn!(
                client_id = %req.client_id,
                count,
                limit,
                "Daily ingest quota exceeded"
            );
            return Ok(IngestResponse::quota_exceeded(count, limit));
        }

        self.publish(req).await
    }

    // ── Batch ingest ──────────────────────────────────────────────────────────

    /// Ingest a batch of API hit events.
    ///
    /// The quota check is **cumulative and upfront**: if the entire batch would
    /// push the client over their daily limit, the whole batch is rejected with
    /// 429. No partial quota consumption occurs on rejection.
    ///
    /// Individual event validation failures are collected and returned in the
    /// `rejected` list; valid events are published (partial-success semantics).
    pub async fn ingest_batch(
        &self,
        events:        Vec<IngestHitRequest>,
        tenant_config: &TenantConfig,
        max_batch_size: usize,
    ) -> Result<BatchIngestResponse, AppError> {
        if events.is_empty() {
            return Err(AppError::bad_request("Batch must contain at least one event"));
        }
        if events.len() > max_batch_size {
            return Err(AppError::bad_request(format!(
                "Batch size {} exceeds maximum of {max_batch_size}",
                events.len()
            )));
        }

        let batch_size = events.len() as i64;

        // Upfront cumulative quota check — reject the whole batch or allow all.
        if let QuotaCheckResult::Exceeded { count, limit } = self
            .quota_tracker
            .check_and_increment(
                // Use the client_id from the first event; all events in a batch
                // must belong to the same client (validated in the handler).
                events.first().map(|e| e.client_id.as_str()).unwrap_or(""),
                tenant_config.daily_ingest_quota,
                batch_size,
            )
        {
            tracing::warn!(
                count,
                limit,
                batch_size,
                "Daily quota exceeded — batch rejected"
            );
            return Ok(BatchIngestResponse::quota_exceeded(count, limit));
        }

        let total = events.len();
        let mut accepted: usize = 0;
        let mut rejected: Vec<BatchEventError> = Vec::new();

        for (index, req) in events.into_iter().enumerate() {
            if let Err(e) = self.validate(&req) {
                rejected.push(BatchEventError {
                    index,
                    reason: e.to_string(),
                });
                continue;
            }

            match self.publish(req).await {
                Ok(_)  => accepted += 1,
                Err(e) => rejected.push(BatchEventError { index, reason: e.to_string() }),
            }
        }

        Ok(BatchIngestResponse { accepted, rejected, total })
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Validate required fields and value ranges.
    fn validate(&self, req: &IngestHitRequest) -> Result<(), AppError> {
        if req.service_name.trim().is_empty() {
            return Err(AppError::bad_request("serviceName is required"));
        }
        if req.endpoint.trim().is_empty() {
            return Err(AppError::bad_request("endpoint is required"));
        }
        if req.client_id.trim().is_empty() {
            return Err(AppError::bad_request("clientId is required"));
        }

        const VALID_METHODS: &[&str] =
            &["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "HEAD"];
        if !VALID_METHODS.contains(&req.method.to_uppercase().as_str()) {
            return Err(AppError::bad_request(format!("Invalid HTTP method: {}", req.method)));
        }

        if req.status_code < 100 || req.status_code > 599 {
            return Err(AppError::bad_request(format!(
                "Invalid status code: {}",
                req.status_code
            )));
        }

        if req.latency_ms < 0.0 {
            return Err(AppError::bad_request(format!(
                "Latency must be non-negative, got: {}",
                req.latency_ms
            )));
        }

        Ok(())
    }

    /// Build and publish a single event to RabbitMQ.
    async fn publish(&self, req: IngestHitRequest) -> Result<IngestResponse, AppError> {
        let event_id = Uuid::new_v4().to_string();
        let now      = Utc::now();

        let envelope = HitEvent {
            event_type: "API_HIT".to_string(),
            data: HitEventData {
                event_id:     event_id.clone(),
                timestamp:    now,
                service_name: req.service_name,
                endpoint:     req.endpoint,
                method:       req.method.to_uppercase(),
                status_code:  req.status_code,
                latency_ms:   req.latency_ms,
                client_id:    req.client_id,
                api_key_id:   req.api_key_id,
                ip:           req.ip.unwrap_or_else(|| "unknown".to_string()),
                user_agent:   req.user_agent.unwrap_or_default(),
            },
            published_at: now,
            attempt:      1,
        };

        match self.producer.publish_api_hit(envelope).await {
            Ok(true) => {
                tracing::info!(event_id = %event_id, "API hit ingested");
                Ok(IngestResponse {
                    event_id,
                    status:    IngestStatus::Queued,
                    timestamp: now,
                    reason:    None,
                })
            }
            Ok(false) => {
                tracing::warn!(event_id = %event_id, "API hit rejected by circuit breaker");
                Ok(IngestResponse {
                    event_id,
                    status:    IngestStatus::Rejected,
                    timestamp: now,
                    reason:    Some("service_unavailable".to_string()),
                })
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to publish API hit");
                Err(AppError::internal("Failed to process API hit"))
            }
        }
    }
}
