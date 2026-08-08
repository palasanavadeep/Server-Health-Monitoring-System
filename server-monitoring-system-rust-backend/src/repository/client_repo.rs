use async_trait::async_trait;
use bson::{doc, oid::ObjectId};
use chrono::Utc;
use mongodb::Database;

use crate::domain::client::Client;
use crate::error::app_error::AppError;

/// Client repository trait.
#[async_trait]
pub trait ClientRepository: Send + Sync {
    async fn create(&self, client: Client) -> Result<Client, AppError>;
    async fn find_by_id(&self, client_id: &str) -> Result<Option<Client>, AppError>;
    async fn find_by_slug(&self, slug: &str) -> Result<Option<Client>, AppError>;
}

/// MongoDB implementation of `ClientRepository`.
pub struct MongoClientRepository {
    collection: mongodb::Collection<Client>,
}

impl MongoClientRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            collection: db.collection::<Client>("clients"),
        }
    }
}

#[async_trait]
impl ClientRepository for MongoClientRepository {
    async fn create(&self, client: Client) -> Result<Client, AppError> {
        let mut client = client;
        client.id = Some(ObjectId::new());
        client.created_at = Some(Utc::now());
        client.updated_at = Some(Utc::now());

        self.collection
            .insert_one(&client)
            .await
            .map_err(|e| {
                let err_str = format!("{}", e);
                if err_str.contains("11000") || err_str.contains("duplicate key") {
                    AppError::conflict("Client already exists")
                } else {
                    AppError::from(e)
                }
            })?;

        tracing::info!(
            mongo_id = ?client.id,
            slug = %client.slug,
            "Client created in MongoDB"
        );

        Ok(client)
    }

    async fn find_by_id(&self, client_id: &str) -> Result<Option<Client>, AppError> {
        let oid = ObjectId::parse_str(client_id)
            .map_err(|_| AppError::bad_request("Invalid client ID format"))?;
        let client = self.collection.find_one(doc! { "_id": oid }).await?;
        Ok(client)
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<Client>, AppError> {
        let client = self.collection.find_one(doc! { "slug": slug }).await?;
        Ok(client)
    }
}
