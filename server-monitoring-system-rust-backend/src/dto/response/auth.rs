//! Auth + user response DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{role::Role, user::UserPermissions};

/// User profile — password field stripped.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    #[serde(rename = "_id")]
    pub id: String,
    pub username: String,
    pub email: String,
    pub role: Role,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
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

/// POST /api/auth/login — full auth response including token.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

/// POST /api/auth/register — response after registering super-admin.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterResponse {
    pub user: UserResponse,
    pub message: String,
}
