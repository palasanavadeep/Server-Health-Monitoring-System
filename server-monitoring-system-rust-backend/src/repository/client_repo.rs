use async_trait::async_trait;
use bson::{doc, oid::ObjectId};
use futures::TryStreamExt;
use mongodb::Database;
use serde::{Deserialize, Serialize};

use crate::domain::client::Client;
use crate::domain::user::default_true;
use crate::error::app_error::AppError;

/// Internal BSON document model for MongoDB `clients` collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClientDocument {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub name: String,
    pub slug: String,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(default = "default_true")]
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<ObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<bson::DateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<bson::DateTime>,
}

impl ClientDocument {
    fn to_domain(self) -> Client {
        Client {
            id: self.id.map(|oid| oid.to_hex()),
            name: self.name,
            slug: self.slug,
            email: self.email,
            description: self.description,
            website: self.website,
            is_active: self.is_active,
            created_by: self.created_by.map(|oid| oid.to_hex()),
            created_at: self.created_at.map(|dt| dt.to_chrono()),
            updated_at: self.updated_at.map(|dt| dt.to_chrono()),
        }
    }

    fn from_domain(client: &Client) -> Self {
        Self {
            id: client
                .id
                .as_deref()
                .and_then(|id| ObjectId::parse_str(id).ok()),
            name: client.name.clone(),
            slug: client.slug.clone(),
            email: client.email.clone(),
            description: client.description.clone(),
            website: client.website.clone(),
            is_active: client.is_active,
            created_by: client
                .created_by
                .as_deref()
                .and_then(|cb| ObjectId::parse_str(cb).ok()),
            created_at: client.created_at.map(bson::DateTime::from_chrono),
            updated_at: client.updated_at.map(bson::DateTime::from_chrono),
        }
    }
}

/// Client repository trait.
#[async_trait]
pub trait ClientRepository: Send + Sync {
    async fn create(&self, client: Client) -> Result<Client, AppError>;
    async fn find_by_id(&self, client_id: &str) -> Result<Option<Client>, AppError>;
    async fn find_by_slug(&self, slug: &str) -> Result<Option<Client>, AppError>;
    async fn find_all(&self) -> Result<Vec<Client>, AppError>;
}

/// MongoDB implementation of `ClientRepository`.
pub struct MongoClientRepository {
    collection: mongodb::Collection<ClientDocument>,
}

impl MongoClientRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            collection: db.collection::<ClientDocument>("clients"),
        }
    }
}

#[async_trait]
impl ClientRepository for MongoClientRepository {
    async fn create(&self, client: Client) -> Result<Client, AppError> {
        let mut doc = ClientDocument::from_domain(&client);
        doc.id = Some(ObjectId::new());
        doc.created_at = Some(bson::DateTime::now());
        doc.updated_at = Some(bson::DateTime::now());

        self.collection.insert_one(&doc).await.map_err(|e| {
            let err_str = format!("{}", e);
            if err_str.contains("11000") || err_str.contains("duplicate key") {
                AppError::conflict("Client already exists")
            } else {
                AppError::from(e)
            }
        })?;

        tracing::info!(
            mongo_id = ?doc.id,
            slug = %doc.slug,
            "Client created in MongoDB"
        );

        Ok(doc.to_domain())
    }

    async fn find_by_id(&self, client_id: &str) -> Result<Option<Client>, AppError> {
        let oid = ObjectId::parse_str(client_id)
            .map_err(|_| AppError::bad_request("Invalid client ID format"))?;
        let doc = self.collection.find_one(doc! { "_id": oid }).await?;
        Ok(doc.map(|d| d.to_domain()))
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<Client>, AppError> {
        let doc = self.collection.find_one(doc! { "slug": slug }).await?;
        Ok(doc.map(|d| d.to_domain()))
    }

    async fn find_all(&self) -> Result<Vec<Client>, AppError> {
        let mut cursor = self.collection.find(doc! {}).await?;
        let mut clients = Vec::new();
        while let Some(doc) = cursor.try_next().await? {
            clients.push(doc.to_domain());
        }
        Ok(clients)
    }
}
