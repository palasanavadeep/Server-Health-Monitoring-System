//! Auth + user response DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::role::Role;
use crate::domain::user::{User, UserPermissions};

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

/// Convert a domain `User` into a `UserResponse`, stripping the password.
///
/// This keeps the domain layer free of DTO imports — the conversion
/// lives at the DTO boundary where `UserResponse` is defined.
impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id.unwrap_or_default(),
            username: user.username,
            email: user.email,
            role: user.role,
            client_id: user.client_id,
            permissions: user.permissions,
            is_active: user.is_active,
            last_login: user.last_login,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
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
