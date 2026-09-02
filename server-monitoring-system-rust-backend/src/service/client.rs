use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::api_key::{ApiKey, ApiKeyPermissions, ApiKeySecurity};
use crate::domain::client::Client;
use crate::domain::role::{self, Role};
use crate::domain::updates::ApiKeyUpdate;
use crate::domain::user::{User, UserPermissions};
use crate::dto::request::client::{
    CreateApiKeyRequest, CreateClientRequest, CreateClientUserRequest, RotateApiKeyRequest,
    UpdateApiKeyRequest,
};
use crate::dto::response::auth::UserResponse;
use crate::error::app_error::AppError;
use crate::repository::api_key_repo::ApiKeyRepository;
use crate::repository::client_repo::ClientRepository;
use crate::repository::user_repo::UserRepository;

/// Client management service — pure business logic working with domain entities and DTOs.
pub struct ClientService {
    client_repository: Arc<dyn ClientRepository>,
    api_key_repository: Arc<dyn ApiKeyRepository>,
    user_repository: Arc<dyn UserRepository>,
}

impl ClientService {
    pub fn new(
        client_repository: Arc<dyn ClientRepository>,
        api_key_repository: Arc<dyn ApiKeyRepository>,
        user_repository: Arc<dyn UserRepository>,
    ) -> Self {
        Self {
            client_repository,
            api_key_repository,
            user_repository,
        }
    }

    // ── Private helpers ────────────────────────────────────────────────────────

    /// Generate URL slug from a display name.
    fn generate_slug(name: &str) -> String {
        name.to_lowercase()
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' {
                    c
                } else {
                    ' '
                }
            })
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join("-")
    }

    /// Generate a random API key value: `sm_key_{40 hex chars}`.
    fn generate_api_key() -> String {
        let mut bytes = [0u8; 20];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
        format!("sm_key_{}", hex::encode(bytes))
    }

    /// Check if a user with the given role may access the named client.
    fn can_user_access_client(
        user_role: &str,
        user_client_id: Option<&str>,
        client_id: &str,
    ) -> bool {
        if user_role == Role::SuperAdmin.as_str() {
            return true;
        }
        matches!(user_client_id, Some(ucid) if ucid == client_id)
    }

    /// Shared validation + fetch used by API-key mutation methods.
    async fn validate_api_key_access(
        &self,
        client_id: &str,
        key_id: &str,
        user_role: &str,
        user_client_id: Option<&str>,
    ) -> Result<ApiKey, AppError> {
        if !Self::can_user_access_client(user_role, user_client_id, client_id) {
            return Err(AppError::forbidden("Access denied"));
        }

        if user_role != Role::SuperAdmin.as_str() && user_role != Role::ClientAdmin.as_str() {
            return Err(AppError::forbidden(
                "Access denied - Insufficient permissions to manage API keys",
            ));
        }

        let api_key = self
            .api_key_repository
            .find_by_key_id(key_id)
            .await?
            .ok_or_else(|| AppError::not_found("API key not found"))?;

        if api_key.client_id != client_id {
            return Err(AppError::bad_request(
                "API key does not belong to this client",
            ));
        }

        Ok(api_key)
    }

    // ── Public service methods ─────────────────────────────────────────────────

    /// Create a new client organization.
    pub async fn create_client(
        &self,
        req: CreateClientRequest,
        admin_user_id: &str,
    ) -> Result<Client, AppError> {
        let slug = Self::generate_slug(&req.name);

        if self.client_repository.find_by_slug(&slug).await?.is_some() {
            return Err(AppError::bad_request(format!(
                "Client with slug {} already exists",
                slug
            )));
        }

        let client = Client {
            id: None,
            name: req.name,
            slug,
            email: req.email,
            description: req.description,
            website: req.website,
            is_active: true,
            created_by: Some(admin_user_id.to_string()),
            created_at: None,
            updated_at: None,
        };

        self.client_repository.create(client).await
    }

    /// Create a user scoped to a client organization.
    pub async fn create_client_user(
        &self,
        client_id: &str,
        req: CreateClientUserRequest,
        admin_role: &str,
        admin_client_id: Option<&str>,
    ) -> Result<UserResponse, AppError> {
        let _client = self
            .client_repository
            .find_by_id(client_id)
            .await?
            .ok_or_else(|| AppError::not_found("Client not found"))?;

        if !Self::can_user_access_client(admin_role, admin_client_id, client_id) {
            return Err(AppError::forbidden("Access denied"));
        }

        let role_str = req.role.as_deref().unwrap_or(Role::ClientViewer.as_str());

        if !role::is_valid_client_role(role_str) {
            return Err(AppError::bad_request("Invalid role for client user"));
        }

        let parsed_role: Role = role_str
            .parse()
            .map_err(|_| AppError::bad_request("Invalid role"))?;

        let permissions = if parsed_role == Role::ClientAdmin {
            UserPermissions {
                can_create_api_keys: true,
                can_manage_users: true,
                can_view_analytics: true,
                can_export_data: true,
            }
        } else {
            UserPermissions {
                can_create_api_keys: false,
                can_manage_users: false,
                can_view_analytics: true,
                can_export_data: false,
            }
        };

        let user = User {
            id: None,
            username: req.username,
            email: req.email,
            password: Some(req.password),
            role: parsed_role,
            client_id: Some(client_id.to_string()),
            permissions: Some(permissions),
            is_active: true,
            last_login: None,
            created_at: None,
            updated_at: None,
        };

        let created = self.user_repository.create(user).await?;

        tracing::info!(
            client_id = %client_id,
            user_id = ?created.id,
            role = %role_str,
            "Client user created"
        );

        Ok(created.into())
    }

    /// Get all users belonging to a client organization.
    ///
    /// Super admin can view users of any client.
    /// Client admin / viewer can only view users of their own client.
    pub async fn get_client_users(
        &self,
        client_id: &str,
        user_role: &str,
        user_client_id: Option<&str>,
    ) -> Result<Vec<UserResponse>, AppError> {
        if !Self::can_user_access_client(user_role, user_client_id, client_id) {
            return Err(AppError::forbidden(
                "Access denied - You can only view users for your own client organization",
            ));
        }

        self.client_repository
            .find_by_id(client_id)
            .await?
            .ok_or_else(|| AppError::not_found("Client not found"))?;

        let users = self.user_repository.find_by_client_id(client_id).await?;
        Ok(users.into_iter().map(Into::into).collect())
    }

    /// Create an API key for a client.
    pub async fn create_api_key(
        &self,
        client_id: &str,
        req: CreateApiKeyRequest,
        user_role: &str,
        user_client_id: Option<&str>,
        user_id: &str,
    ) -> Result<ApiKey, AppError> {
        self.client_repository
            .find_by_id(client_id)
            .await?
            .ok_or_else(|| AppError::not_found("Client not found"))?;

        if !Self::can_user_access_client(user_role, user_client_id, client_id) {
            return Err(AppError::forbidden("Access denied"));
        }

        if user_role != Role::SuperAdmin.as_str() && user_role != Role::ClientAdmin.as_str() {
            return Err(AppError::forbidden(
                "Access denied - Only Super Admin and Client Admin can create API keys",
            ));
        }

        let allowed_ips = if req.allowed_ips.is_empty() {
            vec!["*".to_string()]
        } else {
            req.allowed_ips
        };

        let allowed_origins = if req.allowed_origins.is_empty() {
            vec!["*".to_string()]
        } else {
            req.allowed_origins
        };

        let api_key = ApiKey {
            id: None,
            key_id: Uuid::new_v4().to_string(),
            key_value: Self::generate_api_key(),
            client_id: client_id.to_string(),
            name: req.name,
            description: None,
            environment: "production".to_string(),
            permissions: ApiKeyPermissions {
                can_ingest: req.can_ingest,
                can_read_analytics: req.can_read,
                allowed_services: vec![],
            },
            security: ApiKeySecurity {
                allowed_i_ps: allowed_ips,
                allowed_origins,
                last_rotated: Some(Utc::now()),
                rotation_warning_days: 30,
            },
            is_active: true,
            created_by: Some(user_id.to_string()),
            expires_at: Some(Utc::now() + chrono::Duration::hours(24)),
            created_at: None,
            updated_at: None,
        };

        self.api_key_repository.create(api_key).await
    }

    /// Get all API keys for a client.
    pub async fn get_client_api_keys(
        &self,
        client_id: &str,
        user_role: &str,
        user_client_id: Option<&str>,
    ) -> Result<Vec<ApiKey>, AppError> {
        if !Self::can_user_access_client(user_role, user_client_id, client_id) {
            return Err(AppError::forbidden("Access denied to this client"));
        }
        let keys = self.api_key_repository.find_by_client_id(client_id).await?;
        Ok(keys.into_iter().map(|k| k.to_masked()).collect())
    }

    /// Look up client + key by raw API key value (used by middleware).
    pub async fn get_client_by_api_key(
        &self,
        api_key_value: &str,
    ) -> Result<Option<(Client, ApiKey)>, AppError> {
        let result = self
            .api_key_repository
            .find_by_key_value(api_key_value, false)
            .await?;

        match result {
            Some((key, client)) => {
                if key.is_expired() {
                    return Ok(None);
                }
                Ok(Some((client, key)))
            }
            None => Ok(None),
        }
    }

    /// Update an API key using typed `UpdateApiKeyRequest`.
    pub async fn update_api_key(
        &self,
        client_id: &str,
        key_id: &str,
        req: UpdateApiKeyRequest,
        user_role: &str,
        user_client_id: Option<&str>,
    ) -> Result<Option<ApiKey>, AppError> {
        self.validate_api_key_access(client_id, key_id, user_role, user_client_id)
            .await?;

        let updates = ApiKeyUpdate {
            name: req.name,
            allowed_ips: req.allowed_ips,
            allowed_origins: req.allowed_origins,
            can_ingest: req.can_ingest,
            can_read_analytics: req.can_read,
            ..Default::default()
        };

        let updated = self.api_key_repository
            .update_by_key_id(key_id, updates)
            .await?;
        Ok(updated.map(|k| k.to_masked()))
    }

    /// Delete an API key.
    pub async fn delete_api_key(
        &self,
        client_id: &str,
        key_id: &str,
        user_role: &str,
        user_client_id: Option<&str>,
    ) -> Result<bool, AppError> {
        self.validate_api_key_access(client_id, key_id, user_role, user_client_id)
            .await?;
        self.api_key_repository.delete_by_key_id(key_id).await
    }

    /// Toggle API key active status.
    pub async fn toggle_api_key_status(
        &self,
        client_id: &str,
        key_id: &str,
        is_active: bool,
        user_role: &str,
        user_client_id: Option<&str>,
    ) -> Result<Option<ApiKey>, AppError> {
        self.validate_api_key_access(client_id, key_id, user_role, user_client_id)
            .await?;
        let updates = ApiKeyUpdate {
            is_active: Some(is_active),
            ..Default::default()
        };
        let updated = self.api_key_repository
            .update_by_key_id(key_id, updates)
            .await?;
        Ok(updated.map(|k| k.to_masked()))
    }

    /// Rotate an API key (generate a new key value).
    pub async fn rotate_api_key(
        &self,
        client_id: &str,
        key_id: &str,
        _req: RotateApiKeyRequest,
        user_role: &str,
        user_client_id: Option<&str>,
    ) -> Result<Option<ApiKey>, AppError> {
        self.validate_api_key_access(client_id, key_id, user_role, user_client_id)
            .await?;

        let new_key_value = Self::generate_api_key();
        let updates = ApiKeyUpdate {
            key_value: Some(new_key_value),
            last_rotated: Some(Utc::now()),
            ..Default::default()
        };

        self.api_key_repository
            .update_by_key_id(key_id, updates)
            .await
    }

    /// Get API key details.
    pub async fn get_api_key_details(
        &self,
        client_id: &str,
        key_id: &str,
        user_role: &str,
        user_client_id: Option<&str>,
    ) -> Result<ApiKey, AppError> {
        let key = self.validate_api_key_access(client_id, key_id, user_role, user_client_id)
            .await?;
        Ok(key.to_masked())
    }
}
