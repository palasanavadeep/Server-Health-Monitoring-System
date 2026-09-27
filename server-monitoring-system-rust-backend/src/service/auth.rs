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

    /// Check if caller is authorized to deactivate or activate the target user.
    ///
    /// Rules:
    /// - `super_admin` can manage any user.
    /// - `client_admin` can manage users within their own client organization (but cannot modify super_admin).
    /// - The user themselves (`caller_user_id == target_user_id`) can manage their own account.
    /// - All others are forbidden.
    fn can_manage_user_status(
        caller_user_id: &str,
        caller_role: &str,
        caller_client_id: Option<&str>,
        target_user: &User,
    ) -> bool {
        if caller_role == Role::SuperAdmin.as_str() {
            return true;
        }

        if let Some(ref target_id) = target_user.id {
            if target_id == caller_user_id {
                return true;
            }
        }

        if caller_role == Role::ClientAdmin.as_str() && target_user.role != Role::SuperAdmin {
            if let (Some(caller_cid), Some(target_cid)) = (caller_client_id, target_user.client_id.as_deref()) {
                return caller_cid == target_cid;
            }
        }

        false
    }

    /// Deactivate a user account with strict authorization checks.
    pub async fn deactivate_user(
        &self,
        target_user_id: &str,
        caller_user_id: &str,
        caller_role: &str,
        caller_client_id: Option<&str>,
    ) -> Result<UserResponse, AppError> {
        let target_user = self
            .user_repository
            .find_by_id(target_user_id)
            .await?
            .ok_or_else(|| AppError::not_found("User not found"))?;

        if !Self::can_manage_user_status(caller_user_id, caller_role, caller_client_id, &target_user) {
            return Err(AppError::forbidden(
                "Access denied - Only super_admin, client_admin of the organization, or the user themselves can deactivate this account",
            ));
        }

        let updated = self
            .user_repository
            .deactivate(target_user_id)
            .await?
            .ok_or_else(|| AppError::not_found("User not found"))?;

        Ok(updated.into())
    }

    /// Activate a user account with strict authorization checks.
    pub async fn activate_user(
        &self,
        target_user_id: &str,
        caller_user_id: &str,
        caller_role: &str,
        caller_client_id: Option<&str>,
    ) -> Result<UserResponse, AppError> {
        let target_user = self
            .user_repository
            .find_by_id(target_user_id)
            .await?
            .ok_or_else(|| AppError::not_found("User not found"))?;

        if !Self::can_manage_user_status(caller_user_id, caller_role, caller_client_id, &target_user) {
            return Err(AppError::forbidden(
                "Access denied - Only super_admin, client_admin of the organization, or the user themselves can activate this account",
            ));
        }

        let updated = self
            .user_repository
            .activate(target_user_id)
            .await?
            .ok_or_else(|| AppError::not_found("User not found"))?;

        Ok(updated.into())
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

        let is_super_admin    = user.role == Role::SuperAdmin;
        let can_view_analytics = is_super_admin
            || user.permissions.as_ref().map(|p| p.can_view_analytics).unwrap_or(false);

        let claims = JwtClaims {
            user_id,
            email:              user.email.clone(),
            username:           user.username.clone(),
            role:               user.role.to_string(),
            client_id:          user.client_id.clone(),
            is_super_admin,
            can_view_analytics,
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
    ///
    /// Supported formats: `24h`, `7d`, `90m`.
    /// Falls back to 24 hours on unrecognised input and emits a `WARN` log so
    /// that misconfigured `JWT_EXPIRES_IN` values are visible in production.
    fn parse_expires_in(&self, expires_in: &str) -> Duration {
        if expires_in.ends_with('h') {
            match expires_in.trim_end_matches('h').parse::<i64>() {
                Ok(hours) => return Duration::hours(hours),
                Err(_) => tracing::warn!(
                    value = %expires_in,
                    "JWT_EXPIRES_IN has invalid numeric part (expected e.g. '24h'); defaulting to 24 h"
                ),
            }
        } else if expires_in.ends_with('d') {
            match expires_in.trim_end_matches('d').parse::<i64>() {
                Ok(days) => return Duration::days(days),
                Err(_) => tracing::warn!(
                    value = %expires_in,
                    "JWT_EXPIRES_IN has invalid numeric part (expected e.g. '7d'); defaulting to 24 h"
                ),
            }
        } else if expires_in.ends_with('m') {
            match expires_in.trim_end_matches('m').parse::<i64>() {
                Ok(minutes) => return Duration::minutes(minutes),
                Err(_) => tracing::warn!(
                    value = %expires_in,
                    "JWT_EXPIRES_IN has invalid numeric part (expected e.g. '60m'); defaulting to 24 h"
                ),
            }
        } else {
            tracing::warn!(
                value = %expires_in,
                "JWT_EXPIRES_IN has unrecognised format (expected h/d/m suffix, e.g. '24h'); defaulting to 24 h"
            );
        }
        Duration::hours(24)
    }
}

