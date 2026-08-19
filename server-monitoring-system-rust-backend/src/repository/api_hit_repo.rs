use async_trait::async_trait;
use bson::{doc, oid::ObjectId};
use mongodb::Database;
use serde::{Deserialize, Serialize};

use crate::domain::api_hit::ApiHit;
use crate::error::app_error::AppError;

/// Internal BSON document model for MongoDB `apihits` collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiHitDocument {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub event_id: String,
    pub client_id: ObjectId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_id: Option<ObjectId>,
    pub service_name: String,
    pub endpoint: String,
    pub method: String,
    pub status_code: i32,
    pub latency_ms: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<bson::DateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<bson::DateTime>,
}

impl ApiHitDocument {
    fn from_domain(hit: &ApiHit) -> Self {
        Self {
            id: hit.id.as_deref().and_then(|id| ObjectId::parse_str(id).ok()),
            event_id: hit.event_id.clone(),
            client_id: ObjectId::parse_str(&hit.client_id).unwrap_or_else(|_| ObjectId::new()),
            api_key_id: hit
                .api_key_id
                .as_deref()
                .and_then(|ak| ObjectId::parse_str(ak).ok()),
            service_name: hit.service_name.clone(),
            endpoint: hit.endpoint.clone(),
            method: hit.method.clone(),
            status_code: hit.status_code,
            latency_ms: hit.latency_ms,
            ip: hit.ip.clone(),
            user_agent: hit.user_agent.clone(),
            timestamp: hit.timestamp.map(bson::DateTime::from_chrono),
            created_at: hit.created_at.map(bson::DateTime::from_chrono),
        }
    }
}

/// API hit repository trait — accepts typed `&ApiHit` domain entity directly.
#[async_trait]
pub trait ApiHitRepository: Send + Sync {
    /// Persist a raw API hit.
    ///
    /// Returns `true` if saved, `false` if the event_id was a duplicate (idempotent).
    async fn save(&self, hit: &ApiHit) -> Result<bool, AppError>;

    /// Delete hits with a timestamp older than `before`.
    async fn delete_old_hits(&self, before: chrono::DateTime<chrono::Utc>) -> Result<u64, AppError>;
}

/// MongoDB implementation of `ApiHitRepository`.
pub struct MongoApiHitRepository {
    collection: mongodb::Collection<ApiHitDocument>,
}

impl MongoApiHitRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            collection: db.collection::<ApiHitDocument>("apihits"),
        }
    }
}

#[async_trait]
impl ApiHitRepository for MongoApiHitRepository {
    async fn save(&self, hit: &ApiHit) -> Result<bool, AppError> {
        let doc = ApiHitDocument::from_domain(hit);
        match self.collection.insert_one(&doc).await {
            Ok(_) => {
                tracing::info!(event_id = %hit.event_id, "API hit saved to MongoDB");
                Ok(true)
            }
            Err(e) => {
                let err_str = format!("{}", e);
                if err_str.contains("11000") || err_str.contains("duplicate key") {
                    tracing::warn!(event_id = %hit.event_id, "Duplicate event ID, skipping save");
                    Ok(false)
                } else {
                    tracing::error!("Error saving API hit: {}", e);
                    Err(AppError::from(e))
                }
            }
        }
    }

    async fn delete_old_hits(&self, before: chrono::DateTime<chrono::Utc>) -> Result<u64, AppError> {
        let bson_before = bson::DateTime::from_chrono(before);
        let result = self
            .collection
            .delete_many(doc! { "timestamp": { "$lt": bson_before } })
            .await?;

        tracing::info!(count = result.deleted_count, "Deleted old API hits");
        Ok(result.deleted_count)
    }
}
