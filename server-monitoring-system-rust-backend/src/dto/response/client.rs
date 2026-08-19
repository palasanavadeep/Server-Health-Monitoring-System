//! Client and API-key response DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── Client ────────────────────────────────────────────────────────────────────

/// Single client organisation in responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientResponse {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    pub slug: String,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

// ── API Key ───────────────────────────────────────────────────────────────────

/// API key in list/detail responses (key value is masked after creation).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyResponse {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    pub client_id: String,
    /// Full key is only returned on create / rotate — otherwise masked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    pub allowed_ips: Vec<String>,
    pub allowed_origins: Vec<String>,
    pub can_ingest: bool,
    pub can_read: bool,
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Response returned after onboarding a new client (includes first API key).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardClientResponse {
    pub client: ClientResponse,
    pub api_key: ApiKeyResponse,
}
