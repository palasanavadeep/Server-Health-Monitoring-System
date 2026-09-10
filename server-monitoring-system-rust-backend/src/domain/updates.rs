//! Typed update structs for repository layer.
//!
//! These replace raw `bson::Document` / `serde_json::Value` parameters in
//! repository trait boundaries, keeping infrastructure types out of the
//! service layer.

/// Fields that can be updated on a user profile.
///
/// All fields are optional — only `Some` values are written.
#[derive(Debug, Default)]
pub struct UserProfileUpdate {
    pub username: Option<String>,
    pub email: Option<String>,
}

impl UserProfileUpdate {
    /// Returns `true` if no fields are set.
    pub fn is_empty(&self) -> bool {
        self.username.is_none() && self.email.is_none()
    }
}

use chrono::{DateTime, Utc};

/// Fields that can be updated on an API key.
///
/// All fields are optional — only `Some` values are written.
#[derive(Debug, Default)]
pub struct ApiKeyUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub environment: Option<String>,
    pub key_value: Option<String>,
    pub is_active: Option<bool>,
    pub allowed_ips: Option<Vec<String>>,
    pub allowed_origins: Option<Vec<String>>,
    pub can_ingest: Option<bool>,
    pub can_read_analytics: Option<bool>,
    pub expires_at: Option<Option<DateTime<Utc>>>,
    pub rotation_warning_days: Option<u32>,
    pub last_rotated: Option<DateTime<Utc>>,
}
