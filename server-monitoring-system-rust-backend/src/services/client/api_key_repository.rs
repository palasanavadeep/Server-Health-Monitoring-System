use mongodb::Database;
use bson::{doc, oid::ObjectId};
use chrono::Utc;
use futures::TryStreamExt;

use crate::models::api_key::ApiKey;
use crate::models::client::Client;
use crate::errors::app_error::AppError;

/// ApiKeyRepository - MongoDB operations for ApiKey collection.
/// Mirrors Node.js MongoApiKeyRepository.
pub struct ApiKeyRepository {
    collection: mongodb::Collection<ApiKey>,
    client_collection: mongodb::Collection<Client>,
}

impl ApiKeyRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            collection: db.collection::<ApiKey>("apikeys"),
            client_collection: db.collection::<Client>("clients"),
        }
    }

    /// Create a new API key.
    pub async fn create(&self, api_key: ApiKey) -> Result<ApiKey, AppError> {
        let mut api_key = api_key;
        api_key.id = Some(ObjectId::new());
        api_key.created_at = Some(Utc::now());
        api_key.updated_at = Some(Utc::now());

        self.collection
            .insert_one(&api_key)
            .await
            .map_err(AppError::from)?;

        tracing::info!(key_id = %api_key.key_id, "API key created in database");
        Ok(api_key)
    }

    /// Find API key by key value (with optional inactive filter).
    /// Also populates client data (mirrors Mongoose .populate('clientId')).
    pub async fn find_by_key_value(
        &self,
        key_value: &str,
        include_inactive: bool,
    ) -> Result<Option<(ApiKey, Client)>, AppError> {
        let mut filter = doc! { "keyValue": key_value };
        if !include_inactive {
            filter.insert("isActive", true);
        }

        let api_key = self.collection.find_one(filter).await?;

        match api_key {
            Some(key) => {
                // Populate client
                let client = self
                    .client_collection
                    .find_one(doc! { "_id": key.client_id })
                    .await?;

                match client {
                    Some(c) => Ok(Some((key, c))),
                    None => Ok(None),
                }
            }
            None => Ok(None),
        }
    }

    /// Find API keys by client ID.
    pub async fn find_by_client_id(
        &self,
        client_id: &str,
    ) -> Result<Vec<ApiKey>, AppError> {
        let oid = ObjectId::parse_str(client_id)
            .map_err(|_| AppError::bad_request("Invalid client ID format"))?;

        let mut cursor = self
            .collection
            .find(doc! { "clientId": oid })
            .sort(doc! { "createdAt": -1 })
            .await?;

        let mut keys = Vec::new();
        while let Some(key) = cursor.try_next().await? {
            keys.push(key);
        }

        Ok(keys)
    }

    /// Find API key by keyId (UUID).
    pub async fn find_by_key_id(&self, key_id: &str) -> Result<Option<ApiKey>, AppError> {
        let api_key = self
            .collection
            .find_one(doc! { "keyId": key_id })
            .await?;
        Ok(api_key)
    }

    /// Update API key by keyId.
    pub async fn update_by_key_id(
        &self,
        key_id: &str,
        update_data: bson::Document,
    ) -> Result<Option<ApiKey>, AppError> {
        let result = self
            .collection
            .find_one_and_update(
                doc! { "keyId": key_id },
                doc! { "$set": update_data },
            )
            .return_document(mongodb::options::ReturnDocument::After)
            .await?;

        Ok(result)
    }

    /// Delete API key by keyId.
    pub async fn delete_by_key_id(&self, key_id: &str) -> Result<bool, AppError> {
        let result = self
            .collection
            .delete_one(doc! { "keyId": key_id })
            .await?;
        Ok(result.deleted_count > 0)
    }
}
