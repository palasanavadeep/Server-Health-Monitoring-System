use async_trait::async_trait;
use bson::{doc, oid::ObjectId, Document};
use chrono::Utc;
use mongodb::Database;

use crate::domain::user::User;
use crate::error::app_error::AppError;

/// User repository trait — abstracts data access for testability.
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, user: User) -> Result<User, AppError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError>;
    async fn find_by_id(&self, user_id: &str) -> Result<Option<User>, AppError>;
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError>;
    async fn update_profile(&self, user_id: &str, updates: Document) -> Result<Option<User>, AppError>;
    async fn deactivate(&self, user_id: &str) -> Result<Option<User>, AppError>;
    async fn update_last_login(&self, user_id: &str) -> Result<(), AppError>;
    async fn count(&self, filter: Document) -> Result<u64, AppError>;
}

/// MongoDB implementation of `UserRepository`.
pub struct MongoUserRepository {
    collection: mongodb::Collection<User>,
}

impl MongoUserRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            collection: db.collection::<User>("users"),
        }
    }
}

#[async_trait]
impl UserRepository for MongoUserRepository {
    async fn create(&self, user: User) -> Result<User, AppError> {
        let mut user = user;
        user.id = Some(ObjectId::new());
        user.created_at = Some(Utc::now());
        user.updated_at = Some(Utc::now());

        // Hash password before saving
        if let Some(ref password) = user.password {
            let hashed = bcrypt::hash(password, 10)
                .map_err(|e| AppError::internal(format!("Password hashing failed: {}", e)))?;
            user.password = Some(hashed);
        }

        self.collection
            .insert_one(&user)
            .await
            .map_err(|e| {
                let err_str = format!("{}", e);
                if err_str.contains("11000") || err_str.contains("duplicate key") {
                    AppError::conflict("User already exists")
                } else {
                    AppError::from(e)
                }
            })?;

        tracing::info!("User created in MongoDB: {:?}", user.id);
        Ok(user)
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let user = self.collection.find_one(doc! { "email": email }).await?;
        Ok(user)
    }

    async fn find_by_id(&self, user_id: &str) -> Result<Option<User>, AppError> {
        let oid = ObjectId::parse_str(user_id)
            .map_err(|_| AppError::bad_request("Invalid user ID format"))?;
        let user = self.collection.find_one(doc! { "_id": oid }).await?;
        Ok(user)
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
        let user = self.collection.find_one(doc! { "username": username }).await?;
        Ok(user)
    }

    async fn update_profile(
        &self,
        user_id: &str,
        updates: Document,
    ) -> Result<Option<User>, AppError> {
        let oid = ObjectId::parse_str(user_id)
            .map_err(|_| AppError::bad_request("Invalid user ID format"))?;

        let mut update_doc = updates;
        update_doc.insert("updatedAt", Utc::now());

        let result = self
            .collection
            .find_one_and_update(
                doc! { "_id": oid },
                doc! { "$set": update_doc },
            )
            .return_document(mongodb::options::ReturnDocument::After)
            .await?;

        Ok(result)
    }

    async fn deactivate(&self, user_id: &str) -> Result<Option<User>, AppError> {
        let oid = ObjectId::parse_str(user_id)
            .map_err(|_| AppError::bad_request("Invalid user ID format"))?;

        let result = self
            .collection
            .find_one_and_update(
                doc! { "_id": oid },
                doc! { "$set": { "isActive": false, "updatedAt": Utc::now() } },
            )
            .return_document(mongodb::options::ReturnDocument::After)
            .await?;

        Ok(result)
    }

    async fn update_last_login(&self, user_id: &str) -> Result<(), AppError> {
        let oid = ObjectId::parse_str(user_id)
            .map_err(|_| AppError::bad_request("Invalid user ID format"))?;

        self.collection
            .update_one(
                doc! { "_id": oid },
                doc! { "$set": { "lastLogin": Utc::now() } },
            )
            .await?;

        Ok(())
    }

    async fn count(&self, filter: Document) -> Result<u64, AppError> {
        let count = self.collection.count_documents(filter).await?;
        Ok(count)
    }
}
