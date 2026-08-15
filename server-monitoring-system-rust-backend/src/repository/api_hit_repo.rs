use async_trait::async_trait;
use bson::doc;
use mongodb::Database;

use crate::domain::api_hit::ApiHit;
use crate::error::app_error::AppError;

/// API hit repository trait — `save` accepts a typed `&ApiHit` directly,
/// eliminating the serde_json::Value deserialization overhead on the consumer hot path.
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
    collection: mongodb::Collection<ApiHit>,
}

impl MongoApiHitRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            collection: db.collection::<ApiHit>("apihits"),
        }
    }
}

#[async_trait]
impl ApiHitRepository for MongoApiHitRepository {
    async fn save(&self, hit: &ApiHit) -> Result<bool, AppError> {
        match self.collection.insert_one(hit).await {
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
        let result = self
            .collection
            .delete_many(doc! { "timestamp": { "$lt": before } })
            .await?;

        tracing::info!(count = result.deleted_count, "Deleted old API hits");
        Ok(result.deleted_count)
    }
}
