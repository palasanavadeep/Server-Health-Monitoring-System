use actix_web::{http::StatusCode, HttpResponse};
use serde_json::Value;


use crate::util::response::ResponseFormatter;

/// Centralized application error type.
///
/// Uses `thiserror` for ergonomic error derivation. Each variant maps to
/// an HTTP status code and carries a human-readable message. The optional
/// `errors` field holds structured validation details.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{message}")]
    BadRequest {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("{message}")]
    Unauthorized {
        message: String,
    },

    #[error("{message}")]
    Forbidden {
        message: String,
    },

    #[error("{message}")]
    NotFound {
        message: String,
    },

    #[error("{message}")]
    Conflict {
        message: String,
    },

    #[error("{message}")]
    Internal {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("{message}")]
    WithErrors {
        message: String,
        status_code: u16,
        errors: Value,
    },

    /// 500 — database-layer error (wraps SeaORM / MongoDB errors before reaching the handler).
    #[error("Database error: {0}")]
    Database(String),
}

impl AppError {
    // ── Constructors ────────────────────────────────────────────────

    /// 400 Bad Request
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest {
            message: message.into(),
            source: None,
        }
    }

    /// 401 Unauthorized
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::Unauthorized {
            message: message.into(),
        }
    }

    /// 403 Forbidden
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::Forbidden {
            message: message.into(),
        }
    }

    /// 404 Not Found
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound {
            message: message.into(),
        }
    }

    /// 409 Conflict
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict {
            message: message.into(),
        }
    }

    /// 500 Internal Server Error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
            source: None,
        }
    }

    // ── Accessors ───────────────────────────────────────────────────

    /// Get the HTTP status code for this error.
    pub fn status_code(&self) -> u16 {
        match self {
            Self::BadRequest { .. } => 400,
            Self::Unauthorized { .. } => 401,
            Self::Forbidden { .. } => 403,
            Self::NotFound { .. } => 404,
            Self::Conflict { .. } => 409,
            Self::Database(_) => 500,
            Self::Internal { .. } => 500,
            Self::WithErrors { status_code, .. } => *status_code,
        }
    }

    /// Get the error message.
    pub fn message(&self) -> &str {
        match self {
            Self::BadRequest { message, .. }
            | Self::Unauthorized { message }
            | Self::Forbidden { message }
            | Self::NotFound { message }
            | Self::Conflict { message }
            | Self::Internal { message, .. }
            | Self::WithErrors { message, .. } => message,
            Self::Database(msg) => msg,
        }
    }

    /// Convert to an HTTP response using the standard response format.
    pub fn to_response(&self) -> HttpResponse {
        let status = StatusCode::from_u16(self.status_code())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        let errors = match self {
            Self::WithErrors { errors, .. } => Some(errors.clone()),
            _ => None,
        };

        let body = ResponseFormatter::error(self.message(), self.status_code(), errors);
        HttpResponse::build(status).json(body)
    }
}

// ── actix-web integration ───────────────────────────────────────────

impl actix_web::ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        StatusCode::from_u16(AppError::status_code(self))
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }

    fn error_response(&self) -> HttpResponse {
        self.to_response()
    }
}

// ── From implementations for external error types ───────────────────

impl From<mongodb::error::Error> for AppError {
    fn from(err: mongodb::error::Error) -> Self {
        tracing::error!("MongoDB error: {}", err);

        let err_str = format!("{}", err);
        if err_str.contains("11000") || err_str.contains("duplicate key") {
            return AppError::conflict("Duplicate key error");
        }

        AppError::internal("Internal server error")
    }
}

impl From<sea_orm::DbErr> for AppError {
    fn from(err: sea_orm::DbErr) -> Self {
        tracing::error!("SeaORM/PostgreSQL error: {}", err);
        AppError::Database(err.to_string())
    }
}

impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        use jsonwebtoken::errors::ErrorKind;
        match err.kind() {
            ErrorKind::ExpiredSignature => AppError::unauthorized("Token expired"),
            _ => AppError::unauthorized("Invalid token"),
        }
    }
}
