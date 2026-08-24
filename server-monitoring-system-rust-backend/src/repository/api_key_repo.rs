use async_trait::async_trait;
use bson::{doc, oid::ObjectId};
use futures::TryStreamExt;
use mongodb::Database;
use serde::{Deserialize, Serialize};

use crate::domain::api_key::{ApiKey, ApiKeyPermissions, ApiKeySecurity};
use crate::domain::client::Client;
use crate::domain::updates::ApiKeyUpdate;
use crate::domain::user::default_true;
use crate::error::app_error::AppError;

use super::client_repo::ClientRepository;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiKeySecurityDocument {
    #[serde(default)]
    pub allowed_i_ps: Vec<String>,
    #[serde(default)]
    pub allowed_origins: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_rotated: Option<bson::DateTime>,
    #[serde(default)]
    pub rotation_warning_days: u32,
}

impl ApiKeySecurityDocument {
    fn to_domain(self) -> ApiKeySecurity {
        ApiKeySecurity {
            allowed_i_ps: self.allowed_i_ps,
            allowed_origins: self.allowed_origins,
            last_rotated: self.last_rotated.map(|dt| dt.to_chrono()),
            rotation_warning_days: self.rotation_warning_days,
        }
    }

    fn from_domain(sec: &ApiKeySecurity) -> Self {
        Self {
            allowed_i_ps: sec.allowed_i_ps.clone(),
            allowed_origins: sec.allowed_origins.clone(),
            last_rotated: sec.last_rotated.map(bson::DateTime::from_chrono),
            rotation_warning_days: sec.rotation_warning_days,
        }
    }
}

/// Internal BSON document model for MongoDB `apikeys` collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiKeyDocument {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub key_id: String,
    pub key_value: String,
    pub client_id: ObjectId,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub environment: String,
    pub permissions: ApiKeyPermissions,
    pub security: ApiKeySecurityDocument,
    #[serde(default = "default_true")]
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<ObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<bson::DateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<bson::DateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<bson::DateTime>,
}

impl ApiKeyDocument {
    fn to_domain(self) -> ApiKey {
        ApiKey {
            id: self.id.map(|oid| oid.to_hex()),
            key_id: self.key_id,
            key_value: self.key_value,
            client_id: self.client_id.to_hex(),
            name: self.name,
            description: self.description,
            environment: self.environment,
            permissions: self.permissions,
            security: self.security.to_domain(),
            is_active: self.is_active,
            created_by: self.created_by.map(|oid| oid.to_hex()),
            expires_at: self.expires_at.map(|dt| dt.to_chrono()),
            created_at: self.created_at.map(|dt| dt.to_chrono()),
            updated_at: self.updated_at.map(|dt| dt.to_chrono()),
        }
    }

    fn from_domain(key: &ApiKey) -> Self {
        Self {
            id: key
                .id
                .as_deref()
                .and_then(|id| ObjectId::parse_str(id).ok()),
            key_id: key.key_id.clone(),
            key_value: key.key_value.clone(),
            client_id: ObjectId::parse_str(&key.client_id).unwrap_or_else(|_| ObjectId::new()),
            name: key.name.clone(),
            description: key.description.clone(),
            environment: key.environment.clone(),
            permissions: key.permissions.clone(),
            security: ApiKeySecurityDocument::from_domain(&key.security),
            is_active: key.is_active,
            created_by: key
                .created_by
                .as_deref()
                .and_then(|cb| ObjectId::parse_str(cb).ok()),
            expires_at: key.expires_at.map(bson::DateTime::from_chrono),
            created_at: key.created_at.map(bson::DateTime::from_chrono),
            updated_at: key.updated_at.map(bson::DateTime::from_chrono),
        }
    }
}

/// API key repository trait.
#[async_trait]
pub trait ApiKeyRepository: Send + Sync {
    async fn create(&self, api_key: ApiKey) -> Result<ApiKey, AppError>;
    async fn find_by_key_value(
        &self,
        key_value: &str,
        include_inactive: bool,
    ) -> Result<Option<(ApiKey, Client)>, AppError>;
    async fn find_by_client_id(&self, client_id: &str) -> Result<Vec<ApiKey>, AppError>;
    async fn find_by_key_id(&self, key_id: &str) -> Result<Option<ApiKey>, AppError>;
    async fn update_by_key_id(
        &self,
        key_id: &str,
        update_data: ApiKeyUpdate,
    ) -> Result<Option<ApiKey>, AppError>;
    async fn delete_by_key_id(&self, key_id: &str) -> Result<bool, AppError>;
}

/// MongoDB implementation of `ApiKeyRepository`.
pub struct MongoApiKeyRepository {
    collection: mongodb::Collection<ApiKeyDocument>,
    client_repo: super::client_repo::MongoClientRepository,
}

impl MongoApiKeyRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            collection: db.collection::<ApiKeyDocument>("apikeys"),
            client_repo: super::client_repo::MongoClientRepository::new(db),
        }
    }
}

#[async_trait]
impl ApiKeyRepository for MongoApiKeyRepository {
    async fn create(&self, api_key: ApiKey) -> Result<ApiKey, AppError> {
        let mut doc = ApiKeyDocument::from_domain(&api_key);
        doc.id = Some(ObjectId::new());
        doc.created_at = Some(bson::DateTime::now());
        doc.updated_at = Some(bson::DateTime::now());

        self.collection
            .insert_one(&doc)
            .await
            .map_err(AppError::from)?;

        tracing::info!(key_id = %doc.key_id, "API key created in database");
        Ok(doc.to_domain())
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

        let key_doc = self.collection.find_one(filter).await?;

        match key_doc {
            Some(doc) => {
                let client = self.client_repo.find_by_id(&doc.client_id.to_hex()).await?;

                match client {
                    Some(c) => Ok(Some((doc.to_domain(), c))),
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
        while let Some(doc) = cursor.try_next().await? {
            keys.push(doc.to_domain());
        }

        Ok(keys)
    }

    async fn find_by_key_id(&self, key_id: &str) -> Result<Option<ApiKey>, AppError> {
        let doc = self.collection.find_one(doc! { "keyId": key_id }).await?;
        Ok(doc.map(|d| d.to_domain()))
    }

    async fn update_by_key_id(
        &self,
        key_id: &str,
        update_data: ApiKeyUpdate,
    ) -> Result<Option<ApiKey>, AppError> {
        let mut set_doc = bson::Document::new();

        if let Some(name) = update_data.name {
            set_doc.insert("name", name);
        }
        if let Some(key_value) = update_data.key_value {
            set_doc.insert("keyValue", key_value);
        }
        if let Some(is_active) = update_data.is_active {
            set_doc.insert("isActive", is_active);
        }
        if let Some(ips) = update_data.allowed_ips {
            set_doc.insert("security.allowedIPs", ips);
        }
        if let Some(origins) = update_data.allowed_origins {
            set_doc.insert("security.allowedOrigins", origins);
        }
        if let Some(can_ingest) = update_data.can_ingest {
            set_doc.insert("permissions.canIngest", can_ingest);
        }
        if let Some(can_read) = update_data.can_read_analytics {
            set_doc.insert("permissions.canReadAnalytics", can_read);
        }
        if let Some(rotated_at) = update_data.last_rotated {
            set_doc.insert(
                "security.lastRotated",
                bson::DateTime::from_chrono(rotated_at),
            );
        }
        set_doc.insert("updatedAt", bson::DateTime::now());

        let result = self
            .collection
            .find_one_and_update(doc! { "keyId": key_id }, doc! { "$set": set_doc })
            .return_document(mongodb::options::ReturnDocument::After)
            .await?;

        Ok(result.map(|d| d.to_domain()))
    }

    async fn delete_by_key_id(&self, key_id: &str) -> Result<bool, AppError> {
        let result = self.collection.delete_one(doc! { "keyId": key_id }).await?;
        Ok(result.deleted_count > 0)
    }
}
