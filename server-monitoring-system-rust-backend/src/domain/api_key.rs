use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::user::default_true;

/// API key entity (pure domain model, no DB-specific types).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKey {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    pub key_id: String,
    pub key_value: String,

    pub client_id: String,

    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default = "default_environment")]
    pub environment: String,

    #[serde(default)]
    pub permissions: ApiKeyPermissions,

    #[serde(default)]
    pub security: ApiKeySecurity,

    #[serde(default = "default_true")]
    pub is_active: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Permissions governing what an API key can do.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyPermissions {
    #[serde(default = "default_true")]
    pub can_ingest: bool,
    #[serde(default)]
    pub can_read_analytics: bool,
    #[serde(default)]
    pub allowed_services: Vec<String>,
}

/// Security constraints for an API key (IP/origin allow-lists, rotation).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeySecurity {
    #[serde(default = "default_allowed_ips")]
    pub allowed_i_ps: Vec<String>,
    #[serde(default = "default_allowed_origins")]
    pub allowed_origins: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_rotated: Option<DateTime<Utc>>,
    #[serde(default = "default_rotation_warning_days")]
    pub rotation_warning_days: u32,
}

fn default_environment() -> String {
    "production".to_string()
}

fn default_allowed_ips() -> Vec<String> {
    vec!["0.0.0.0/0".to_string()]
}

fn default_allowed_origins() -> Vec<String> {
    vec!["*".to_string()]
}

fn default_rotation_warning_days() -> u32 {
    30
}

impl ApiKey {
    /// Check if this API key has expired.
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expires) => Utc::now() > expires,
            None => false,
        }
    }
}

/// API key joined with its owning client data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyWithClient {
    #[serde(flatten)]
    pub api_key: ApiKey,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub client: Option<super::client::Client>,
}
