use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use std::sync::Arc;

use crate::config::settings::AppConfig;
use crate::domain::role::Role;
use crate::domain::updates::UserProfileUpdate;
use crate::domain::user::{JwtClaims, User};
use crate::dto::response::auth::UserResponse;
use crate::error::app_error::AppError;
use crate::repository::user_repo::UserRepository;
use crate::util::security::SecurityUtils;

/// Authentication service — handles user registration, login, and profile management.
///
/// Encapsulates all auth business logic using pure domain models and DTOs.
pub struct AuthService {
    user_repository: Arc<dyn UserRepository>,
    config: AppConfig,
}

impl AuthService {
    pub fn new(user_repository: Arc<dyn UserRepository>, config: AppConfig) -> Self {
        Self {
            user_repository,
            config,
        }
    }

    /// Onboard the first super admin (only when no users exist).
    #[tracing::instrument(skip(self, password), fields(email = %email))]
    pub async fn onboard_super_admin(
        &self,
        username: &str,
        email: &str,
        password: &str,
    ) -> Result<UserResponse, AppError> {
        let count = self.user_repository.count_all().await?;
        if count > 0 {
            return Err(AppError::bad_request("Super admin already exists"));
        }

        let validation = SecurityUtils::validate_password(password, &self.config.password_policy);
        if !validation.success {
            return Err(AppError::bad_request(validation.errors.join(", ")));
        }

        let user = User {
            id: None,
            username: username.to_string(),
            email: email.to_string(),
            password: Some(password.to_string()),
            role: Role::SuperAdmin,
            client_id: None,
            permissions: None,
            is_active: true,
            last_login: None,
            created_at: None,
            updated_at: None,
        };

        let created = self.user_repository.create(user).await?;
        Ok(created.into())
    }

    /// Register a new user (admin-only operation).
    #[tracing::instrument(skip(self, password), fields(email = %email, role = %role))]
    pub async fn register(
        &self,
        username: &str,
        email: &str,
        password: &str,
        role: &str,
    ) -> Result<UserResponse, AppError> {
        let validation = SecurityUtils::validate_password(password, &self.config.password_policy);
        if !validation.success {
            return Err(AppError::bad_request(validation.errors.join(", ")));
        }

        if self.user_repository.find_by_email(email).await?.is_some() {
            return Err(AppError::conflict("User with this email already exists"));
        }

        let parsed_role: Role = role
            .parse()
            .map_err(|_| AppError::bad_request("Invalid role"))?;

        let user = User {
            id: None,
            username: username.to_string(),
            email: email.to_string(),
            password: Some(password.to_string()),
            role: parsed_role,
            client_id: None,
            permissions: None,
            is_active: true,
            last_login: None,
            created_at: None,
            updated_at: None,
        };

        let created = self.user_repository.create(user).await?;
        Ok(created.into())
    }

    /// Login and return JWT token.
    #[tracing::instrument(skip(self, password), fields(email = %email))]
    pub async fn login(
        &self,
        email: &str,
        password: &str,
    ) -> Result<(UserResponse, String), AppError> {
        let user = self
            .user_repository
            .find_by_email(email)
            .await?
            .ok_or_else(|| AppError::unauthorized("Invalid credentials"))?;

        if !user.is_active {
            return Err(AppError::forbidden("Account is deactivated"));
        }

        let stored_password = user
            .password
            .as_ref()
            .ok_or_else(|| AppError::internal("No password set"))?;

        let is_valid = bcrypt::verify(password, stored_password)
            .map_err(|e| AppError::internal(format!("Password verification failed: {}", e)))?;

        if !is_valid {
            return Err(AppError::unauthorized("Invalid credentials"));
        }

        let user_id_str = user.id.clone().unwrap_or_default();
        self.user_repository.update_last_login(&user_id_str).await?;

        let token = self.generate_token(&user)?;
        Ok((user.into(), token))
    }

    /// Get user profile by ID.
    pub async fn get_profile(&self, user_id: &str) -> Result<UserResponse, AppError> {
        let user = self
            .user_repository
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::not_found("User not found"))?;
        Ok(user.into())
    }

    /// Update user profile fields.
    pub async fn update_profile(
        &self,
        user_id: &str,
        updates: UserProfileUpdate,
    ) -> Result<UserResponse, AppError> {
        if updates.is_empty() {
            return Err(AppError::bad_request("No valid fields to update"));
        }

        let user = self
            .user_repository
            .update_profile(user_id, updates)
            .await?
            .ok_or_else(|| AppError::not_found("User not found"))?;

        Ok(user.into())
    }

    /// Deactivate a user account.
    pub async fn deactivate_user(&self, user_id: &str) -> Result<UserResponse, AppError> {
        let user = self
            .user_repository
            .deactivate(user_id)
            .await?
            .ok_or_else(|| AppError::not_found("User not found"))?;
        Ok(user.into())
    }

    /// Check if user has super_admin role.
    pub async fn check_super_admin_permissions(&self, user_id: &str) -> Result<bool, AppError> {
        let user = self
            .user_repository
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::not_found("User not found"))?;
        Ok(user.role == Role::SuperAdmin)
    }

    /// Access the service configuration.
    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    /// Generate a JWT token for a user.
    fn generate_token(&self, user: &User) -> Result<String, AppError> {
        let user_id = user.id.clone().unwrap_or_default();

        let now = Utc::now();
        let expires_in = self.parse_expires_in(&self.config.jwt.expires_in);
        let exp = now + expires_in;

        let claims = JwtClaims {
            user_id,
            email: user.email.clone(),
            username: user.username.clone(),
            role: user.role.to_string(),
            client_id: user.client_id.clone(),
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

    /// Parse JWT expires_in string to Duration.
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
            Duration::hours(24)
        }
    }
}
