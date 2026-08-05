use serde::{Deserialize, Serialize};
use bson::oid::ObjectId;
use chrono::{DateTime, Utc};

/// ApiHit model mirroring Node.js ApiHits.model.js (Mongoose schema).
/// Raw API hit events stored in MongoDB.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiHit {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,

    pub event_id: String,

    pub client_id: ObjectId,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_id: Option<ObjectId>,

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
