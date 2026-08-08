use bson::oid::ObjectId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::role::Role;

/// Shared default helper for boolean `true` fields.
pub(crate) fn default_true() -> bool {
    true
}

/// User entity stored in MongoDB `users` collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
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
    pub last_login: Option<DateTime<Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Granular permissions for client-scoped users.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPermissions {
    #[serde(default)]
    pub can_create_api_keys: bool,
    #[serde(default)]
    pub can_manage_users: bool,
    #[serde(default = "default_true")]
    pub can_view_analytics: bool,
    #[serde(default)]
    pub can_export_data: bool,
}

/// User response DTO — strips sensitive fields (password).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    pub username: String,
    pub email: String,
    pub role: Role,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<ObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<UserPermissions>,
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_login: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

impl User {
    /// Convert to a response DTO, stripping the password.
    pub fn to_response(&self) -> UserResponse {
        UserResponse {
            id: self.id.unwrap_or_else(ObjectId::new),
            username: self.username.clone(),
            email: self.email.clone(),
            role: self.role,
            client_id: self.client_id,
            permissions: self.permissions.clone(),
            is_active: self.is_active,
            last_login: self.last_login,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// JWT claims payload embedded in authentication tokens.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JwtClaims {
    pub user_id: String,
    pub email: String,
    pub username: String,
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    pub iat: i64,
    pub exp: i64,
}
