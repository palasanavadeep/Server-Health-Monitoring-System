use chrono::{Utc, Duration};
use jsonwebtoken::{encode, EncodingKey, Header};
use bson::doc;

use crate::config::app_config::AppConfig;
use crate::constants::roles::ApplicationRoles;
use crate::errors::app_error::AppError;
use crate::models::user::{JwtClaims, User, UserResponse};
use crate::utils::security_utils::SecurityUtils;
use super::repository::UserRepository;

/// AuthService - Business logic for authentication.
/// Mirrors Node.js AuthService class exactly.
pub struct AuthService {
    user_repository: UserRepository,
    config: AppConfig,
}

impl AuthService {
    pub fn new(user_repository: UserRepository, config: AppConfig) -> Self {
        Self {
            user_repository,
            config,
        }
    }

    /// Onboard the first super admin (only when no users exist).
    /// Mirrors Node.js onboardSuperAdmin().
    pub async fn onboard_super_admin(
        &self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<UserResponse, AppError> {
        // Check if any users exist
        let count = self.user_repository.count(doc! {}).await?;
        if count > 0 {
            return Err(AppError::bad_request("Super admin already exists"));
        }

        // Validate password
        let validation = SecurityUtils::validate_password(password);
        if !validation.success {
            return Err(AppError::bad_request(validation.errors.join(", ")));
        }

        let user = User {
            id: None,
            username: username.to_string(),
            email: email.to_string(),
            password: Some(password.to_string()),
            role: ApplicationRoles::SUPER_ADMIN.to_string(),
            client_id: None,
            permissions: None,
            is_active: true,
            last_login: None,
            created_at: None,
            updated_at: None,
        };

        let created = self.user_repository.create(user).await?;
        Ok(created.to_response())
    }

    /// Register a new user (admin-only operation).
    /// Mirrors Node.js register().
    pub async fn register(
        &self,
        username: &str,
        email: &str,
        password: &str,
        role: &str,
    ) -> Result<UserResponse, AppError> {
        // Validate password
        let validation = SecurityUtils::validate_password(password);
        if !validation.success {
            return Err(AppError::bad_request(validation.errors.join(", ")));
        }

        // Check existing user
        if let Some(_) = self.user_repository.find_by_email(email).await? {
            return Err(AppError::conflict("User with this email already exists"));
        }

        let user = User {
            id: None,
            username: username.to_string(),
            email: email.to_string(),
            password: Some(password.to_string()),
            role: role.to_string(),
            client_id: None,
            permissions: None,
            is_active: true,
            last_login: None,
            created_at: None,
            updated_at: None,
        };

        let created = self.user_repository.create(user).await?;
        Ok(created.to_response())
    }

    /// Login and return JWT token.
    /// Mirrors Node.js login(). Note: preserves the typo "Invliad Credentials".
    pub async fn login(
        &self,
        email: &str,
        password: &str,
    ) -> Result<(UserResponse, String), AppError> {
        let user = self
            .user_repository
            .find_by_email(email)
            .await?
            .ok_or_else(|| AppError::unauthorized("Invliad Credentials"))?;

        if !user.is_active {
            return Err(AppError::forbidden("Account is deactivated"));
        }

        // Verify password
        let stored_password = user
            .password
            .as_ref()
            .ok_or_else(|| AppError::internal("No password set"))?;

        let is_valid = bcrypt::verify(password, stored_password)
            .map_err(|e| AppError::internal(format!("Password verification failed: {}", e)))?;

        if !is_valid {
            return Err(AppError::unauthorized("Invliad Credentials"));
        }

        // Update last login
        let user_id_str = user.id.map(|id| id.to_hex()).unwrap_or_default();
        self.user_repository.update_last_login(&user_id_str).await?;

        // Generate JWT
        let token = self.generate_token(&user)?;

        Ok((user.to_response(), token))
    }

    /// Get user profile.
    /// Mirrors Node.js getProfile().
    pub async fn get_profile(&self, user_id: &str) -> Result<UserResponse, AppError> {
        let user = self
            .user_repository
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::not_found("User not found"))?;

        Ok(user.to_response())
    }

    /// Update user profile.
    /// Mirrors Node.js updateProfile().
    pub async fn update_profile(
        &self,
        user_id: &str,
        updates: serde_json::Value,
    ) -> Result<UserResponse, AppError> {
        let mut update_doc = bson::Document::new();

        if let Some(username) = updates.get("username").and_then(|v| v.as_str()) {
            update_doc.insert("username", username);
        }
        if let Some(email) = updates.get("email").and_then(|v| v.as_str()) {
            update_doc.insert("email", email);
        }

        if update_doc.is_empty() {
            return Err(AppError::bad_request("No valid fields to update"));
        }

        let user = self
            .user_repository
            .update_profile(user_id, update_doc)
            .await?
            .ok_or_else(|| AppError::not_found("User not found"))?;

        Ok(user.to_response())
    }

    /// Deactivate a user.
    /// Mirrors Node.js deactivateUser().
    pub async fn deactivate_user(&self, user_id: &str) -> Result<UserResponse, AppError> {
        let user = self
            .user_repository
            .deactivate(user_id)
            .await?
            .ok_or_else(|| AppError::not_found("User not found"))?;

        Ok(user.to_response())
    }

    /// Check if user has super_admin permissions.
    /// Mirrors Node.js checkSuperAdminPermissions().
    pub async fn check_super_admin_permissions(&self, user_id: &str) -> Result<bool, AppError> {
        let user = self
            .user_repository
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::not_found("User not found"))?;

        Ok(user.role == ApplicationRoles::SUPER_ADMIN)
    }

    /// Generate a JWT token for a user.
    fn generate_token(&self, user: &User) -> Result<String, AppError> {
        let user_id = user
            .id
            .map(|id| id.to_hex())
            .unwrap_or_default();

        let now = Utc::now();
        let expires_in = self.parse_expires_in(&self.config.jwt.expires_in);
        let exp = now + expires_in;

        let claims = JwtClaims {
            user_id,
            email: user.email.clone(),
            username: user.username.clone(),
            role: user.role.clone(),
            client_id: user.client_id.map(|id| id.to_hex()),
            iat: now.timestamp(),
            exp: exp.timestamp(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.config.jwt.secret.as_bytes()),
        )?;

        Ok(token)
    }

    /// Parse JWT expires_in string (e.g., "24h", "7d") to Duration.
    fn parse_expires_in(&self, expires_in: &str) -> Duration {
        if expires_in.ends_with('h') {
            let hours: i64 = expires_in.trim_end_matches('h').parse().unwrap_or(24);
            Duration::hours(hours)
        } else if expires_in.ends_with('d') {
            let days: i64 = expires_in.trim_end_matches('d').parse().unwrap_or(1);
            Duration::days(days)
        } else if expires_in.ends_with('m') {
            let minutes: i64 = expires_in.trim_end_matches('m').parse().unwrap_or(60);
            Duration::minutes(minutes)
        } else {
            Duration::hours(24) // default
        }
    }
}
