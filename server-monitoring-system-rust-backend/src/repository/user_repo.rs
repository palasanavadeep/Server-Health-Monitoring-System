use async_trait::async_trait;
use bson::{doc, oid::ObjectId};
use mongodb::Database;
use serde::{Deserialize, Serialize};

use crate::domain::role::Role;
use crate::domain::updates::UserProfileUpdate;
use crate::domain::user::{default_true, User, UserPermissions};
use crate::error::app_error::AppError;

/// Internal BSON document model for MongoDB `users` collection.
///
/// Keeps BSON-specific types (ObjectId, bson::DateTime) encapsulated within
/// the repository layer so that domain entities remain pure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserDocument {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default)]
    pub role: Role,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<ObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<UserPermissions>,
    #[serde(default = "default_true")]
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_login: Option<bson::DateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<bson::DateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<bson::DateTime>,
}

impl UserDocument {
    fn to_domain(self) -> User {
        User {
            id: self.id.map(|oid| oid.to_hex()),
            username: self.username,
            email: self.email,
            password: self.password,
            role: self.role,
            client_id: self.client_id.map(|oid| oid.to_hex()),
            permissions: self.permissions,
            is_active: self.is_active,
            last_login: self.last_login.map(|dt| dt.to_chrono()),
            created_at: self.created_at.map(|dt| dt.to_chrono()),
            updated_at: self.updated_at.map(|dt| dt.to_chrono()),
        }
    }

    fn from_domain(user: &User) -> Self {
        Self {
            id: user
                .id
                .as_deref()
                .and_then(|id| ObjectId::parse_str(id).ok()),
            username: user.username.clone(),
            email: user.email.clone(),
            password: user.password.clone(),
            role: user.role,
            client_id: user
                .client_id
                .as_deref()
                .and_then(|cid| ObjectId::parse_str(cid).ok()),
            permissions: user.permissions.clone(),
            is_active: user.is_active,
            last_login: user.last_login.map(bson::DateTime::from_chrono),
            created_at: user.created_at.map(bson::DateTime::from_chrono),
            updated_at: user.updated_at.map(bson::DateTime::from_chrono),
        }
    }
}

/// User repository trait — abstracts data access for testability.
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, user: User) -> Result<User, AppError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError>;
    async fn find_by_id(&self, user_id: &str) -> Result<Option<User>, AppError>;
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError>;
    async fn update_profile(
        &self,
        user_id: &str,
        updates: UserProfileUpdate,
    ) -> Result<Option<User>, AppError>;
    async fn deactivate(&self, user_id: &str) -> Result<Option<User>, AppError>;
    async fn update_last_login(&self, user_id: &str) -> Result<(), AppError>;
    async fn count_all(&self) -> Result<u64, AppError>;
}

/// MongoDB implementation of `UserRepository`.
pub struct MongoUserRepository {
    collection: mongodb::Collection<UserDocument>,
}

impl MongoUserRepository {
    pub fn new(db: &Database) -> Self {
        Self {
            collection: db.collection::<UserDocument>("users"),
        }
    }
}

#[async_trait]
impl UserRepository for MongoUserRepository {
    async fn create(&self, user: User) -> Result<User, AppError> {
        let mut doc = UserDocument::from_domain(&user);
        doc.id = Some(ObjectId::new());
        doc.created_at = Some(bson::DateTime::now());
        doc.updated_at = Some(bson::DateTime::now());

        // Hash password before saving
        if let Some(ref password) = doc.password {
            let hashed = bcrypt::hash(password, 10)
                .map_err(|e| AppError::internal(format!("Password hashing failed: {}", e)))?;
            doc.password = Some(hashed);
        }

        self.collection.insert_one(&doc).await.map_err(|e| {
            let err_str = format!("{}", e);
            if err_str.contains("11000") || err_str.contains("duplicate key") {
                AppError::conflict("User already exists")
            } else {
                AppError::from(e)
            }
        })?;

        tracing::info!("User created in MongoDB: {:?}", doc.id);
        Ok(doc.to_domain())
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let doc = self.collection.find_one(doc! { "email": email }).await?;
        Ok(doc.map(|d| d.to_domain()))
    }

    async fn find_by_id(&self, user_id: &str) -> Result<Option<User>, AppError> {
        let oid = ObjectId::parse_str(user_id)
            .map_err(|_| AppError::bad_request("Invalid user ID format"))?;
        let doc = self.collection.find_one(doc! { "_id": oid }).await?;
        Ok(doc.map(|d| d.to_domain()))
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
        let doc = self
            .collection
            .find_one(doc! { "username": username })
            .await?;
        Ok(doc.map(|d| d.to_domain()))
    }

    async fn update_profile(
        &self,
        user_id: &str,
        updates: UserProfileUpdate,
    ) -> Result<Option<User>, AppError> {
        let oid = ObjectId::parse_str(user_id)
            .map_err(|_| AppError::bad_request("Invalid user ID format"))?;

        let mut update_doc = bson::Document::new();
        if let Some(username) = updates.username {
            update_doc.insert("username", username);
        }
        if let Some(email) = updates.email {
            update_doc.insert("email", email);
        }
        update_doc.insert("updatedAt", bson::DateTime::now());

        let result = self
            .collection
            .find_one_and_update(doc! { "_id": oid }, doc! { "$set": update_doc })
            .return_document(mongodb::options::ReturnDocument::After)
            .await?;

        Ok(result.map(|d| d.to_domain()))
    }

    async fn deactivate(&self, user_id: &str) -> Result<Option<User>, AppError> {
        let oid = ObjectId::parse_str(user_id)
            .map_err(|_| AppError::bad_request("Invalid user ID format"))?;

        let result = self
            .collection
            .find_one_and_update(
                doc! { "_id": oid },
                doc! { "$set": { "isActive": false, "updatedAt": bson::DateTime::now() } },
            )
            .return_document(mongodb::options::ReturnDocument::After)
            .await?;

        Ok(result.map(|d| d.to_domain()))
    }

    async fn update_last_login(&self, user_id: &str) -> Result<(), AppError> {
        let oid = ObjectId::parse_str(user_id)
            .map_err(|_| AppError::bad_request("Invalid user ID format"))?;

        self.collection
            .update_one(
                doc! { "_id": oid },
                doc! { "$set": { "lastLogin": bson::DateTime::now() } },
            )
            .await?;

        Ok(())
    }

    async fn count_all(&self) -> Result<u64, AppError> {
        let count = self.collection.count_documents(doc! {}).await?;
        Ok(count)
    }
}
