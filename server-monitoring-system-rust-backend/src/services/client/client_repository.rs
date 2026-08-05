use mongodb::Database;
use bson::{doc, oid::ObjectId};
use chrono::Utc;

use crate::models::client::Client;
use crate::errors::app_error::AppError;

/// ClientRepository - MongoDB operations for Client collection.
/// Mirrors Node.js MongoClientRepository.
pub struct ClientRepository {
    collection: mongodb::Collection<Client>,
}

impl ClientRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            collection: db.collection::<Client>("clients"),
        }
    }

    /// Create a new client.
    pub async fn create(&self, client: Client) -> Result<Client, AppError> {
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

    /// Find client by ID.
    pub async fn find_by_id(&self, client_id: &str) -> Result<Option<Client>, AppError> {
        let oid = ObjectId::parse_str(client_id)
            .map_err(|_| AppError::bad_request("Invalid client ID format"))?;
        let client = self
            .collection
            .find_one(doc! { "_id": oid })
            .await?;
        Ok(client)
    }

    /// Find client by slug.
    pub async fn find_by_slug(&self, slug: &str) -> Result<Option<Client>, AppError> {
        let client = self
            .collection
            .find_one(doc! { "slug": slug })
            .await?;
        Ok(client)
    }
}
