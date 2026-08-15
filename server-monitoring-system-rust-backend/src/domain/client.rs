use bson::oid::ObjectId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

use super::user::default_true;

/// Client organization entity stored in MongoDB `clients` collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Client {
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
    pub created_at: Option<DateTime<Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

// ── Request DTOs ──────────────────────────────────────────────────────────────

/// Request body for creating a new client organization.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateClientRequest {
    #[validate(length(min = 2, message = "Name must be at least 2 characters"))]
    pub name: String,
    #[validate(email(message = "Invalid email address"))]
    pub email: String,
    pub description: Option<String>,
    pub website: Option<String>,
}

/// Request body for creating a user inside a client organization.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateClientUserRequest {
    #[validate(length(min = 3, message = "Username must be at least 3 characters"))]
    pub username: String,
    #[validate(email(message = "Invalid email address"))]
    pub email: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
    pub role: Option<String>,
}

/// Request body for creating an API key.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiKeyRequest {
    #[validate(length(min = 1, message = "Name is required"))]
    pub name: String,
    #[serde(default)]
    pub allowed_i_ps: Vec<String>,
    #[serde(default)]
    pub allowed_origins: Vec<String>,
    #[serde(default = "default_true")]
    pub can_ingest: bool,
    #[serde(default)]
    pub can_read: bool,
}

/// Request body for updating an existing API key.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApiKeyRequest {
    pub name: Option<String>,
    pub allowed_i_ps: Option<Vec<String>>,
    pub allowed_origins: Option<Vec<String>>,
    pub can_ingest: Option<bool>,
    pub can_read: Option<bool>,
}

/// Request body for rotating an API key.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RotateApiKeyRequest {
    pub reason: Option<String>,
}
