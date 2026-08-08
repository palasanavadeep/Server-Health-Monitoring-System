use async_trait::async_trait;
use bson::{doc, oid::ObjectId, Document};
use chrono::Utc;
use futures::TryStreamExt;
use mongodb::Database;

use crate::domain::api_key::ApiKey;
use crate::domain::client::Client;
use crate::error::app_error::AppError;

/// API key repository trait.
#[async_trait]
pub trait ApiKeyRepository: Send + Sync {
    async fn create(&self, api_key: ApiKey) -> Result<ApiKey, AppError>;
    async fn find_by_key_value(&self, key_value: &str, include_inactive: bool) -> Result<Option<(ApiKey, Client)>, AppError>;
    async fn find_by_client_id(&self, client_id: &str) -> Result<Vec<ApiKey>, AppError>;
    async fn find_by_key_id(&self, key_id: &str) -> Result<Option<ApiKey>, AppError>;
    async fn update_by_key_id(&self, key_id: &str, update_data: Document) -> Result<Option<ApiKey>, AppError>;
    async fn delete_by_key_id(&self, key_id: &str) -> Result<bool, AppError>;
}

/// MongoDB implementation of `ApiKeyRepository`.
pub struct MongoApiKeyRepository {
    collection: mongodb::Collection<ApiKey>,
    client_collection: mongodb::Collection<Client>,
}

impl MongoApiKeyRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            collection: db.collection::<ApiKey>("apikeys"),
            client_collection: db.collection::<Client>("clients"),
        }
    }
}

#[async_trait]
impl ApiKeyRepository for MongoApiKeyRepository {
    async fn create(&self, api_key: ApiKey) -> Result<ApiKey, AppError> {
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

    async fn find_by_key_value(
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

    async fn find_by_client_id(&self, client_id: &str) -> Result<Vec<ApiKey>, AppError> {
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

    async fn find_by_key_id(&self, key_id: &str) -> Result<Option<ApiKey>, AppError> {
        let api_key = self.collection.find_one(doc! { "keyId": key_id }).await?;
        Ok(api_key)
    }

    async fn update_by_key_id(
        &self,
        key_id: &str,
        update_data: Document,
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

    async fn delete_by_key_id(&self, key_id: &str) -> Result<bool, AppError> {
        let result = self
            .collection
            .delete_one(doc! { "keyId": key_id })
            .await?;
        Ok(result.deleted_count > 0)
    }
}
