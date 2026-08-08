use chrono::Utc;
use uuid::Uuid;

use crate::error::app_error::AppError;
use crate::messaging::producer::EventProducer;

/// Ingest service — validates and publishes API hit events to the message queue.
pub struct IngestService {
    event_producer: EventProducer,
}

impl IngestService {
    pub fn new(event_producer: EventProducer) -> Self {
        Self { event_producer }
    }

    /// Ingest an API hit event. Validates, enriches, and publishes to the queue.
    pub async fn ingest_api_hit(
        &self,
        hit_data: serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        self.validate_hit_data(&hit_data)?;

        let event_id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().to_rfc3339();

        let method = hit_data
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_uppercase();

        let status_code: i64 = hit_data
            .get("statusCode")
            .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
            .unwrap_or(0);

        let latency_ms: f64 = hit_data
            .get("latencyMs")
            .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
            .unwrap_or(0.0);

        let event = serde_json::json!({
            "eventId": event_id,
            "timestamp": timestamp,
            "serviceName": hit_data.get("serviceName"),
            "endpoint": hit_data.get("endpoint"),
            "method": method,
            "statusCode": status_code,
            "latencyMs": latency_ms,
            "clientId": hit_data.get("clientId"),
            "apiKeyId": hit_data.get("apiKeyId"),
            "ip": hit_data.get("ip").and_then(|v| v.as_str()).unwrap_or("unknown"),
            "userAgent": hit_data.get("userAgent").and_then(|v| v.as_str()).unwrap_or(""),
        });

        match self.event_producer.publish_api_hit(event).await {
            Ok(true) => {
                tracing::info!(event_id = %event_id, "API hit ingested");
                Ok(serde_json::json!({
                    "eventId": event_id,
                    "status": "queued",
                    "timestamp": timestamp,
                }))
            }
            Ok(false) => {
                tracing::warn!(event_id = %event_id, "API hit rejected by circuit breaker");
                Ok(serde_json::json!({
                    "eventId": event_id,
                    "status": "rejected",
                    "reason": "service_unavailable",
                    "timestamp": timestamp,
                }))
            }
            Err(e) => {
                tracing::error!(error = %e, "Error ingesting API hit");
                Err(AppError::internal("Failed to process API hit"))
            }
        }
    }

    /// Validate required fields and value ranges for hit data.
    fn validate_hit_data(&self, hit_data: &serde_json::Value) -> Result<(), AppError> {
        let required_fields = [
            "serviceName", "endpoint", "method", "statusCode", "latencyMs", "clientId",
        ];

        let missing: Vec<&str> = required_fields
            .iter()
            .filter(|&&field| {
                hit_data.get(field).is_none()
                    || hit_data.get(field) == Some(&serde_json::Value::Null)
                    || hit_data.get(field).and_then(|v| v.as_str()) == Some("")
            })
            .copied()
            .collect();

        if !missing.is_empty() {
            return Err(AppError::bad_request(format!(
                "Missing required fields: {}",
                missing.join(",")
            )));
        }

        // Validate HTTP method
        let valid_methods = ["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "HEAD"];
        let method = hit_data
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_uppercase();

        if !valid_methods.contains(&method.as_str()) {
            return Err(AppError::bad_request(format!(
                "Invalid HTTP methods: {} ",
                method
            )));
        }

        // Validate status code range
        let status_code: i64 = hit_data
            .get("statusCode")
            .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
            .unwrap_or(-1);

        if status_code < 100 || status_code > 599 {
            return Err(AppError::bad_request(format!(
                "Invalid Status code : {} ",
                status_code
            )));
        }

        // Validate latency is non-negative
        let latency: f64 = hit_data
            .get("latencyMs")
            .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
            .unwrap_or(-1.0);

        if latency < 0.0 {
            return Err(AppError::bad_request(format!(
                "Invalid latency : {} ",
                latency
            )));
        }

        Ok(())
    }
}
