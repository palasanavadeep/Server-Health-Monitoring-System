use chrono::Utc;
use uuid::Uuid;

use crate::domain::ingest::{HitEvent, HitEventData};
use crate::dto::request::ingest::IngestHitRequest;
use crate::dto::response::ingest::{IngestResponse, IngestStatus};
use crate::error::app_error::AppError;
use crate::messaging::producer::EventProducer;

/// Ingest service — validates and publishes API hit events to the message queue.
///
/// Accepts `IngestHitRequest` (a typed struct) instead of `serde_json::Value`,
/// eliminating runtime field-lookup overhead on the hot ingest path.
pub struct IngestService {
    event_producer: EventProducer,
}

impl IngestService {
    pub fn new(event_producer: EventProducer) -> Self {
        Self { event_producer }
    }

    /// Ingest an API hit event.
    ///
    /// Validates fields, enriches with timestamp + event_id, and publishes to RabbitMQ.
    /// Returns an `IngestResult` indicating whether the event was queued or rejected.
    #[tracing::instrument(skip(self), fields(service_name = %req.service_name, endpoint = %req.endpoint))]
    pub async fn ingest_api_hit(&self, req: IngestHitRequest) -> Result<IngestResponse, AppError> {
        self.validate(&req)?;

        let event_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let envelope = HitEvent {
            event_type: "API_HIT".to_string(),
            data: HitEventData {
                event_id: event_id.clone(),
                timestamp: now,
                service_name: req.service_name,
                endpoint: req.endpoint,
                method: req.method.to_uppercase(),
                status_code: req.status_code,
                latency_ms: req.latency_ms,
                client_id: req.client_id,
                api_key_id: req.api_key_id,
                ip: req.ip.unwrap_or_else(|| "unknown".to_string()),
                user_agent: req.user_agent.unwrap_or_default(),
            },
            published_at: now,
            attempt: 1,
        };

        match self.event_producer.publish_api_hit(envelope).await {
            Ok(true) => {
                tracing::info!(event_id = %event_id, "API hit ingested");
                Ok(IngestResponse {
                    event_id,
                    status: IngestStatus::Queued,
                    timestamp: now,
                    reason: None,
                })
            }
            Ok(false) => {
                tracing::warn!(event_id = %event_id, "API hit rejected by circuit breaker");
                Ok(IngestResponse {
                    event_id,
                    status: IngestStatus::Rejected,
                    timestamp: now,
                    reason: Some("service_unavailable".to_string()),
                })
            }
            Err(e) => {
                tracing::error!(error = %e, "Error ingesting API hit");
                Err(AppError::internal("Failed to process API hit"))
            }
        }
    }

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

        // Validate HTTP method
        const VALID_METHODS: &[&str] =
            &["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "HEAD"];
        if !VALID_METHODS.contains(&req.method.to_uppercase().as_str()) {
            return Err(AppError::bad_request(format!(
                "Invalid HTTP methods: {}",
                req.method
            )));
        }

        // Validate status code range (100–599)
        if req.status_code < 100 || req.status_code > 599 {
            return Err(AppError::bad_request(format!(
                "Invalid Status code : {}",
                req.status_code
            )));
        }

        // Validate latency is non-negative
        if req.latency_ms < 0.0 {
            return Err(AppError::bad_request(format!(
                "Invalid latency : {}",
                req.latency_ms
            )));
        }

        Ok(())
    }
}
