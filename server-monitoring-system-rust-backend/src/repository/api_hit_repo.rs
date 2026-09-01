use async_trait::async_trait;
use bson::{doc, oid::ObjectId};
use mongodb::{options::IndexOptions, Database, IndexModel};
use serde::{Deserialize, Serialize};

use crate::domain::api_hit::ApiHit;
use crate::error::app_error::AppError;

// ── Internal document model ────────────────────────────────────────────────────

/// Internal BSON document model for the MongoDB `apihits` collection.
///
/// This is a private implementation detail — callers work with the `ApiHit`
/// domain type and the `ApiHitRepository` trait only.
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
            id: hit
                .id
                .as_deref()
                .and_then(|id| ObjectId::parse_str(id).ok()),
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

// ── Trait ──────────────────────────────────────────────────────────────────────

/// API hit repository trait — accepts typed `&ApiHit` domain entities directly.
#[async_trait]
pub trait ApiHitRepository: Send + Sync {
    /// Persist a raw API hit to MongoDB.
    ///
    /// This operation is **idempotent**: if an `ApiHit` with the same `event_id`
    /// already exists (due to RabbitMQ redelivery), the duplicate is silently
    /// ignored and `Ok(false)` is returned. Callers must not treat `Ok(false)` as
    /// an error — it is a normal outcome under at-least-once delivery.
    ///
    /// - `Ok(true)`  — event was newly inserted
    /// - `Ok(false)` — duplicate `event_id`; safely skipped
    /// - `Err`       — genuine infrastructure failure; caller should NACK
    async fn save(&self, hit: &ApiHit) -> Result<bool, AppError>;

    /// Delete hits with a `timestamp` older than `before`.
    async fn delete_old_hits(&self, before: chrono::DateTime<chrono::Utc>)
        -> Result<u64, AppError>;
}

// ── Implementation ─────────────────────────────────────────────────────────────

/// MongoDB implementation of `ApiHitRepository`.
pub struct MongoApiHitRepository {
    collection: mongodb::Collection<ApiHitDocument>,
}

impl MongoApiHitRepository {
    /// Create the repository and ensure the `event_id` unique index exists.
    ///
    /// # Startup-critical
    ///
    /// Returns `Err` if the unique index cannot be created. Callers **must**
    /// propagate this error and abort startup — without this index, duplicate
    /// `event_id` inserts are accepted silently, breaking the idempotency guarantee.
    pub async fn new(
        db: &Database,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let collection = db.collection::<ApiHitDocument>("apihits");

        // Unique index on eventId — the MongoDB idempotency safety net.
        // Duplicate inserts are caught here and treated as success by save().
        let index = IndexModel::builder()
            .keys(doc! { "eventId": 1 })
            .options(
                IndexOptions::builder()
                    .unique(true)
                    .name("uq_event_id".to_string())
                    .build(),
            )
            .build();

        collection
            .create_index(index)
            .await
            .map_err(|e| format!("Failed to create eventId unique index on 'apihits': {e}"))?;

        tracing::info!("MongoDB 'apihits' unique index on eventId verified");

        Ok(Self { collection })
    }
}

#[async_trait]
impl ApiHitRepository for MongoApiHitRepository {
    async fn save(&self, hit: &ApiHit) -> Result<bool, AppError> {
        let doc = ApiHitDocument::from_domain(hit);
        match self.collection.insert_one(&doc).await {
            Ok(_) => {
                tracing::debug!(event_id = %hit.event_id, "API hit saved to MongoDB");
                Ok(true)
            }
            Err(e) => {
                let err_str = format!("{e}");
                if err_str.contains("11000") || err_str.contains("duplicate key") {
                    // Duplicate event_id = RabbitMQ redelivery of an already-persisted event.
                    // This is expected under at-least-once delivery — not an error.
                    tracing::debug!(
                        event_id = %hit.event_id,
                        "Duplicate event_id in MongoDB — idempotent skip"
                    );
                    Ok(false)
                } else {
                    tracing::error!(event_id = %hit.event_id, error = %e, "MongoDB insert failed");
                    Err(AppError::from(e))
                }
            }
        }
    }

    async fn delete_old_hits(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, AppError> {
        let bson_before = bson::DateTime::from_chrono(before);
        let result = self
            .collection
            .delete_many(doc! { "timestamp": { "$lt": bson_before } })
            .await?;

        tracing::info!(count = result.deleted_count, "Deleted old API hits");
        Ok(result.deleted_count)
    }
}
