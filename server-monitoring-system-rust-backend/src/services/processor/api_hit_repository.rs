use bson::{doc, oid::ObjectId};
use mongodb::Database;

use crate::models::api_hit::ApiHit;
use crate::errors::app_error::AppError;

/// ApiHitRepository - Saves raw API hits to MongoDB.
/// Mirrors Node.js ApiHitRepository.
pub struct ApiHitRepository {
    collection: mongodb::Collection<ApiHit>,
}

impl ApiHitRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            collection: db.collection::<ApiHit>("apihits"),
        }
    }

    /// Save a raw API hit event.
    pub async fn save(&self, event_data: serde_json::Value) -> Result<Option<ApiHit>, AppError> {
        let client_id_str = event_data.get("clientId").and_then(|v| v.as_str()).unwrap_or("");
        let client_oid = ObjectId::parse_str(client_id_str).unwrap_or_else(|_| ObjectId::new());

        let api_key_id = event_data
            .get("apiKeyId")
            .and_then(|v| v.as_str())
            .and_then(|s| ObjectId::parse_str(s).ok());

        let hit = ApiHit {
            id: Some(ObjectId::new()),
            event_id: event_data.get("eventId").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            client_id: client_oid,
            api_key_id,
            service_name: event_data.get("serviceName").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            endpoint: event_data.get("endpoint").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            method: event_data.get("method").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            status_code: event_data.get("statusCode").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            latency_ms: event_data.get("latencyMs").and_then(|v| v.as_f64()).unwrap_or(0.0),
            ip: event_data.get("ip").and_then(|v| v.as_str()).map(|s| s.to_string()),
            user_agent: event_data.get("userAgent").and_then(|v| v.as_str()).map(|s| s.to_string()),
            timestamp: event_data.get("timestamp").and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok()),
            created_at: Some(chrono::Utc::now()),
        };

        match self.collection.insert_one(&hit).await {
            Ok(_) => {
                tracing::info!(event_id = %hit.event_id, "API hit saved to MongoDB");
                Ok(Some(hit))
            }
            Err(e) => {
                let err_str = format!("{}", e);
                if err_str.contains("11000") || err_str.contains("duplicate key") {
                    tracing::warn!(event_id = %hit.event_id, "Duplicate event ID, skipping save");
                    Ok(None)
                } else {
                    tracing::error!("Error saving API hit: {}", e);
                    Err(AppError::from(e))
                }
            }
        }
    }

    /// Delete old hits before a date.
    pub async fn delete_old_hits(&self, before: chrono::DateTime<chrono::Utc>) -> Result<u64, AppError> {
        let result = self.collection
            .delete_many(doc! { "timestamp": { "$lt": before } })
            .await?;

        tracing::info!(count = result.deleted_count, "Deleted old API hits");
        Ok(result.deleted_count)
    }
}
