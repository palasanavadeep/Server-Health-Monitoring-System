use actix_web::{HttpResponse, http::StatusCode};
use serde_json::Value;
use std::fmt;

use crate::utils::response_formatter::ResponseFormatter;

/// AppError - Custom error type mirroring Node.js AppError class.
/// Carries statusCode, message, and optional errors array.
#[derive(Debug)]
pub struct AppError {
    pub message: String,
    pub status_code: u16,
    pub errors: Option<Value>,
    pub is_operational: bool,
}

impl AppError {
    pub fn new(message: impl Into<String>, status_code: u16) -> Self {
        Self {
            message: message.into(),
            status_code,
            errors: None,
            is_operational: true,
        }
    }

    pub fn with_errors(mut self, errors: Value) -> Self {
        self.errors = Some(errors);
        self
    }

    /// Bad Request (400)
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(message, 400)
    }

    /// Unauthorized (401)
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(message, 401)
    }

    /// Forbidden (403)
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(message, 403)
    }

    /// Not Found (404)
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(message, 404)
    }

    /// Conflict (409)
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(message, 409)
    }

    /// Internal Server Error (500)
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(message, 500)
    }

    /// Convert to HttpResponse using ResponseFormatter (matches Node.js error handler)
    pub fn to_response(&self) -> HttpResponse {
        let status = StatusCode::from_u16(self.status_code)
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        let body = ResponseFormatter::error(
            &self.message,
            self.status_code,
            self.errors.clone(),
        );

        HttpResponse::build(status).json(body)
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AppError {}

/// Implement actix_web ResponseError so AppError can be returned from handlers
impl actix_web::ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        StatusCode::from_u16(self.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }

    fn error_response(&self) -> HttpResponse {
        self.to_response()
    }
}

/// Convert mongodb errors
impl From<mongodb::error::Error> for AppError {
    fn from(err: mongodb::error::Error) -> Self {
        tracing::error!("MongoDB error: {}", err);

        // Check for duplicate key error (code 11000)
        let err_str = format!("{}", err);
        if err_str.contains("11000") || err_str.contains("duplicate key") {
            return AppError::conflict("Duplicate key error");
        }

        AppError::internal("Internal server error")
    }
}

/// Convert sqlx errors
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!("PostgreSQL error: {}", err);
        AppError::internal("Internal server error")
    }
}

/// Convert jsonwebtoken errors
impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        use jsonwebtoken::errors::ErrorKind;
        match err.kind() {
            ErrorKind::ExpiredSignature => AppError::unauthorized("Token expired"),
            _ => AppError::unauthorized("Invalid token"),
        }
    }
}
