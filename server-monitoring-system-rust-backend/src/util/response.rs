//! Typed API response formatter.
//!
//! All handlers return responses through these types to guarantee a consistent
//! JSON envelope across the entire API surface — matching the Node.js contract:
//!
//! ```json
//! { "success": true, "message": "...", "data": {...}, "statusCode": 201, "timestamp": "..." }
//! { "success": false, "message": "...", "error": {...}, "statusCode": 400, "timestamp": "..." }
//! ```
//!
//! # Usage in handlers
//! ```rust
//! use actix_web::HttpResponse;
//! use crate::util::response::ResponseFormatter;
//!
//! // Success (200)
//! HttpResponse::Ok().json(ResponseFormatter::ok(user, "Profile fetched"))
//!
//! // Created (201)
//! HttpResponse::Created().json(ResponseFormatter::created(client, "Client created"))
//!
//! // Error
//! HttpResponse::BadRequest().json(ResponseFormatter::bad_request("Validation failed", None))
//! ```

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Core response envelopes ───────────────────────────────────────────────────

/// Successful response envelope — `data` is strongly typed.
///
/// Serializes to:
/// ```json
/// {
///   "success": true,
///   "message": "...",
///   "data": <T>,
///   "statusCode": 200,
///   "timestamp": "2026-08-15T18:01:05.882Z"
/// }
/// ```
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub message: String,
    pub data: T,
    pub status_code: u16,
    pub timestamp: String,
}

/// Error response envelope.
///
/// Serializes to:
/// ```json
/// {
///   "success": false,
///   "message": "...",
///   "error": null | {...},
///   "statusCode": 400,
///   "timestamp": "2026-08-15T18:01:05.882Z"
/// }
/// ```
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiErrorResponse {
    pub success: bool,
    pub message: String,
    pub error: Option<Value>,
    pub status_code: u16,
    pub timestamp: String,
}

/// Paginated list response envelope.
///
/// Serializes to:
/// ```json
/// {
///   "success": true,
///   "data": [...],
///   "pagination": { "page": 1, "limit": 20, "total": 100, "totalPages": 5 },
///   "timestamp": "..."
/// }
/// ```
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiPagedResponse<T: Serialize> {
    pub success: bool,
    pub data: Vec<T>,
    pub pagination: PaginationMeta,
    pub timestamp: String,
}

/// Pagination metadata included in paged responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationMeta {
    pub page: i64,
    pub limit: i64,
    pub total: i64,
    pub total_pages: i64,
}

// ── Builder ───────────────────────────────────────────────────────────────────

pub struct ResponseFormatter;

impl ResponseFormatter {
    // ── Timestamp ─────────────────────────────────────────────────────────────

    /// ISO-8601 timestamp with millisecond precision — `"2026-08-15T18:01:05.882Z"`.
    fn timestamp() -> String {
        Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    }

    // ── Success variants ──────────────────────────────────────────────────────

    /// HTTP 200 OK — generic success with a typed data payload.
    pub fn ok<T: Serialize>(data: T, message: &str) -> ApiResponse<T> {
        ApiResponse {
            success: true,
            message: message.to_string(),
            data,
            status_code: 200,
            timestamp: Self::timestamp(),
        }
    }

    /// HTTP 201 Created — resource created successfully.
    pub fn created<T: Serialize>(data: T, message: &str) -> ApiResponse<T> {
        ApiResponse {
            success: true,
            message: message.to_string(),
            data,
            status_code: 201,
            timestamp: Self::timestamp(),
        }
    }

    /// Arbitrary status code success.
    pub fn success<T: Serialize>(data: T, message: &str, status_code: u16) -> ApiResponse<T> {
        ApiResponse {
            success: true,
            message: message.to_string(),
            data,
            status_code,
            timestamp: Self::timestamp(),
        }
    }

    // ── Paginated ─────────────────────────────────────────────────────────────

    /// HTTP 200 OK — paginated list response.
    pub fn paged<T: Serialize>(data: Vec<T>, page: i64, limit: i64, total: i64) -> ApiPagedResponse<T> {
        let total_pages = if limit > 0 {
            (total as f64 / limit as f64).ceil() as i64
        } else {
            0
        };

        ApiPagedResponse {
            success: true,
            data,
            pagination: PaginationMeta {
                page,
                limit,
                total,
                total_pages,
            },
            timestamp: Self::timestamp(),
        }
    }

    // ── Error variants ────────────────────────────────────────────────────────

    /// Generic error with optional detail payload.
    pub fn error(message: &str, status_code: u16, error: Option<Value>) -> ApiErrorResponse {
        ApiErrorResponse {
            success: false,
            message: message.to_string(),
            error,
            status_code,
            timestamp: Self::timestamp(),
        }
    }

    /// HTTP 400 Bad Request.
    pub fn bad_request(message: &str, error: Option<Value>) -> ApiErrorResponse {
        Self::error(message, 400, error)
    }

    /// HTTP 401 Unauthorized.
    pub fn unauthorized(message: &str) -> ApiErrorResponse {
        Self::error(message, 401, None)
    }

    /// HTTP 403 Forbidden.
    pub fn forbidden(message: &str) -> ApiErrorResponse {
        Self::error(message, 403, None)
    }

    /// HTTP 404 Not Found.
    pub fn not_found(message: &str) -> ApiErrorResponse {
        Self::error(message, 404, None)
    }

    /// HTTP 409 Conflict.
    pub fn conflict(message: &str, error: Option<Value>) -> ApiErrorResponse {
        Self::error(message, 409, error)
    }

    /// HTTP 422 Unprocessable Entity — validation failure.
    pub fn validation_error(error: Option<Value>) -> ApiErrorResponse {
        Self::error("Validation failed", 400, error)
    }

    /// HTTP 500 Internal Server Error.
    pub fn internal_error(message: &str) -> ApiErrorResponse {
        Self::error(message, 500, None)
    }
}
