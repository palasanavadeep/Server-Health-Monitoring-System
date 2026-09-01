//! Client & API-key management request DTOs.

use serde::{Deserialize, Serialize};
use validator::Validate;

// ── Client ────────────────────────────────────────────────────────────────────

/// POST /api/admin/clients/onboard — create a new client organisation.
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

/// POST /api/admin/clients/{clientId}/users — add a user to a client.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateClientUserRequest {
    #[validate(length(min = 3, message = "Username must be at least 3 characters"))]
    pub username: String,

    #[validate(email(message = "Invalid email address"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,

    /// Optional role override; defaults to `"client_user"`.
    pub role: Option<String>,
}

// ── API Keys ──────────────────────────────────────────────────────────────────

/// POST /api/admin/clients/{clientId}/api/keys — create an API key.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiKeyRequest {
    #[validate(length(min = 1, message = "Name is required"))]
    pub name: String,

    #[serde(default)]
    pub allowed_ips: Vec<String>,

    #[serde(default)]
    pub allowed_origins: Vec<String>,

    #[serde(default)]
    pub can_ingest: bool,

    #[serde(default = "default_true")]
    pub can_read: bool,
}

/// PUT /api/admin/clients/{clientId}/api/keys/{keyId} — update an API key.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApiKeyRequest {
    pub name: Option<String>,
    pub allowed_ips: Option<Vec<String>>,
    pub allowed_origins: Option<Vec<String>>,
    pub can_ingest: Option<bool>,
    pub can_read: Option<bool>,
}

/// POST /api/admin/clients/{clientId}/api/keys/{keyId}/rotate — rotate secret.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RotateApiKeyRequest {
    /// Optional reason recorded in the audit log.
    pub reason: Option<String>,
}

fn default_true() -> bool {
    true
}
