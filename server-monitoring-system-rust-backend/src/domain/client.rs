//! Client organization — pure business entity.
//!
//! Request DTOs (CreateClientRequest, etc.) live in `dto/request/client.rs`.
//! Response DTOs (ClientResponse, etc.) live in `dto/response/client.rs`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::user::default_true;

/// Client organisation entity (pure domain model, no DB-specific types).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Client {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

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
    pub created_by: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}
