use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Raw API hit event entity (pure domain model, no DB-specific types).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiHit {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    pub event_id: String,

    pub client_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_id: Option<String>,

    pub service_name: String,
    pub endpoint: String,
    pub method: String,
    pub status_code: i32,
    pub latency_ms: f64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
}
