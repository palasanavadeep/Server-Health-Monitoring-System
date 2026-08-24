//! Auth request DTOs — login, registration, and profile updates.
//!
//! This is the **single canonical location** for auth-related request types.
//! Handlers import directly from here.

use serde::{Deserialize, Serialize};
use validator::Validate;

/// POST /api/auth/onboard-super-admin — bootstrap the first super-admin user.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct OnboardSuperAdminRequest {
    #[validate(length(min = 3, message = "Username must be at least 3 characters long"))]
    pub username: String,

    #[validate(email(message = "Please provide a valid email"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters long"))]
    pub password: String,
}

/// POST /api/auth/register — create a new user (admin-only).
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    #[validate(length(min = 3, message = "Username must be at least 3 characters long"))]
    pub username: String,

    #[validate(email(message = "Please provide a valid email"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters long"))]
    pub password: String,

    pub role: Option<String>,
}

/// POST /api/auth/login — authenticate with email + password.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    #[validate(email(message = "Please provide a valid email"))]
    pub email: String,

    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
}

/// PUT /api/auth/profile — update display name or email.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProfileRequest {
    pub username: Option<String>,
    pub email: Option<String>,
}
