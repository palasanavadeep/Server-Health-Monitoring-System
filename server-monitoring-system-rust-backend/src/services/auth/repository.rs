use mongodb::Database;
use bson::{doc, oid::ObjectId};
use chrono::Utc;

use crate::models::user::User;
use crate::errors::app_error::AppError;

/// UserRepository - MongoDB operations for User collection.
/// Mirrors Node.js UserRepository class.
pub struct UserRepository {
    collection: mongodb::Collection<User>,
}

impl UserRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            collection: db.collection::<User>("users"),
        }
    }

    /// Create a new user.
    pub async fn create(&self, user: User) -> Result<User, AppError> {
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

    /// Find user by email.
    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let user = self
            .collection
            .find_one(doc! { "email": email })
            .await?;
        Ok(user)
    }

    /// Find user by ID.
    pub async fn find_by_id(&self, user_id: &str) -> Result<Option<User>, AppError> {
        let oid = ObjectId::parse_str(user_id)
            .map_err(|_| AppError::bad_request("Invalid user ID format"))?;
        let user = self
            .collection
            .find_one(doc! { "_id": oid })
            .await?;
        Ok(user)
    }

    /// Find user by username.
    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
        let user = self
            .collection
            .find_one(doc! { "username": username })
            .await?;
        Ok(user)
    }

    /// Update user profile.
    pub async fn update_profile(
        &self,
        user_id: &str,
        updates: bson::Document,
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

    /// Deactivate a user.
    pub async fn deactivate(&self, user_id: &str) -> Result<Option<User>, AppError> {
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

    /// Update last login time.
    pub async fn update_last_login(&self, user_id: &str) -> Result<(), AppError> {
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

    /// Count documents matching filter.
    pub async fn count(&self, filter: bson::Document) -> Result<u64, AppError> {
        let count = self.collection.count_documents(filter).await?;
        Ok(count)
    }
}
