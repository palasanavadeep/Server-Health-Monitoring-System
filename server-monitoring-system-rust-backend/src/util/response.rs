use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Standardized API response formatting.
///
/// All endpoints return responses through this formatter to ensure
/// consistent JSON structure across the entire API surface.

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
    pub data: Value,
    pub status_code: u16,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub success: bool,
    pub message: String,
    pub error: Value,
    pub status_code: u16,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationMeta {
    pub page: i64,
    pub limit: i64,
    pub total: i64,
    pub total_pages: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResponse {
    pub success: bool,
    pub data: Value,
    pub pagination: PaginationMeta,
    pub timestamp: String,
}

pub struct ResponseFormatter;

impl ResponseFormatter {
    fn timestamp() -> String {
        Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    }

    /// Format a successful response.
    pub fn success(data: Value, message: &str, status_code: u16) -> Value {
        serde_json::json!({
            "success": true,
            "message": message,
            "data": data,
            "statusCode": status_code,
            "timestamp": Self::timestamp()
        })
    }

    /// Shorthand success with defaults (message="Success", code=200).
    pub fn success_default(data: Value) -> Value {
        Self::success(data, "Success", 200)
    }

    /// Format an error response.
    pub fn error(message: &str, status_code: u16, error: Option<Value>) -> Value {
        serde_json::json!({
            "success": false,
            "message": message,
            "error": error.unwrap_or(Value::Null),
            "statusCode": status_code,
            "timestamp": Self::timestamp()
        })
    }

    /// Format a validation error response.
    pub fn validation_error(error: Option<Value>) -> Value {
        serde_json::json!({
            "success": false,
            "message": "Validation failed",
            "error": error.unwrap_or(Value::Null),
            "statusCode": 400,
            "timestamp": Self::timestamp()
        })
    }

    /// Format a paginated response.
    pub fn paginated(data: Value, page: i64, limit: i64, total: i64) -> Value {
        let total_pages = if limit > 0 {
            (total as f64 / limit as f64).ceil() as i64
        } else {
            0
        };

        serde_json::json!({
            "success": true,
            "data": data,
            "pagination": {
                "page": page,
                "limit": limit,
                "total": total,
                "totalPages": total_pages
            },
            "timestamp": Self::timestamp()
        })
    }
}
