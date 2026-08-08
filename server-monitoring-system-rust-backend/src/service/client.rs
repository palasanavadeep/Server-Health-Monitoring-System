use bson::{doc, oid::ObjectId};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::api_key::{ApiKey, ApiKeyPermissions, ApiKeySecurity};
use crate::domain::client::Client;
use crate::domain::role::{self, Role};
use crate::domain::user::{User, UserPermissions, UserResponse};
use crate::error::app_error::AppError;
use crate::repository::api_key_repo::ApiKeyRepository;
use crate::repository::client_repo::ClientRepository;
use crate::repository::user_repo::UserRepository;

/// Client management service — handles clients, client users, and API keys.
///
/// Uses trait-object repositories injected via the constructor for
/// loose coupling and testability.
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

    /// Generate URL slug from name.
    fn generate_slug(name: &str) -> String {
        name.to_lowercase()
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == ' ' || c == '-' {
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

    /// Generate a random API key value: sm_key_{40 hex chars}
    fn generate_api_key() -> String {
        let mut bytes = [0u8; 20];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
        format!("sm_key_{}", hex::encode(bytes))
    }

    /// Check if user can access a client based on role.
    fn can_user_access_client(user_role: &str, user_client_id: Option<&str>, client_id: &str) -> bool {
        if user_role == Role::SuperAdmin.as_str() {
            return true;
        }
        match user_client_id {
            Some(ucid) => ucid == client_id,
            None => false,
        }
    }

    /// Create a new client organization.
    pub async fn create_client(
        &self,
        client_data: serde_json::Value,
        admin_user_id: &str,
    ) -> Result<Client, AppError> {
        let name = client_data
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::bad_request("Name is required"))?;

        let email = client_data
            .get("email")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::bad_request("Email is required"))?;

        let slug = Self::generate_slug(name);

        if self.client_repository.find_by_slug(&slug).await?.is_some() {
            return Err(AppError::bad_request(format!(
                "Client with slug {} already exists",
                slug
            )));
        }

        let admin_oid = ObjectId::parse_str(admin_user_id)
            .map_err(|_| AppError::bad_request("Invalid admin user ID"))?;

        let client = Client {
            id: None,
            name: name.to_string(),
            slug,
            email: email.to_string(),
            description: client_data.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
            website: client_data.get("website").and_then(|v| v.as_str()).map(|s| s.to_string()),
            is_active: true,
            created_by: Some(admin_oid),
            created_at: None,
            updated_at: None,
        };

        self.client_repository.create(client).await
    }

    /// Create a user for a client organization.
    pub async fn create_client_user(
        &self,
        client_id: &str,
        user_data: serde_json::Value,
        admin_role: &str,
        admin_client_id: Option<&str>,
    ) -> Result<UserResponse, AppError> {
        let _client = self.client_repository.find_by_id(client_id).await?
            .ok_or_else(|| AppError::not_found("Client not found"))?;

        if !Self::can_user_access_client(admin_role, admin_client_id, client_id) {
            return Err(AppError::forbidden("Access denied"));
        }

        let username = user_data.get("username").and_then(|v| v.as_str())
            .ok_or_else(|| AppError::bad_request("Username is required"))?;
        let email = user_data.get("email").and_then(|v| v.as_str())
            .ok_or_else(|| AppError::bad_request("Email is required"))?;
        let password = user_data.get("password").and_then(|v| v.as_str())
            .ok_or_else(|| AppError::bad_request("Password is required"))?;
        let role_str = user_data.get("role").and_then(|v| v.as_str())
            .unwrap_or(Role::ClientViewer.as_str());

        if !role::is_valid_client_role(role_str) {
            return Err(AppError::bad_request("Invalid role for client user"));
        }

        let parsed_role: Role = role_str.parse()
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

        let client_oid = ObjectId::parse_str(client_id)
            .map_err(|_| AppError::bad_request("Invalid client ID"))?;

        let user = User {
            id: None,
            username: username.to_string(),
            email: email.to_string(),
            password: Some(password.to_string()),
            role: parsed_role,
            client_id: Some(client_oid),
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

        Ok(created.to_response())
    }

    /// Create an API key for a client.
    pub async fn create_api_key(
        &self,
        client_id: &str,
        key_data: serde_json::Value,
        user_role: &str,
        user_client_id: Option<&str>,
        user_id: &str,
    ) -> Result<ApiKey, AppError> {
        let _client = self.client_repository.find_by_id(client_id).await?
            .ok_or_else(|| AppError::not_found("Client not found"))?;

        if !Self::can_user_access_client(user_role, user_client_id, client_id) {
            return Err(AppError::forbidden("Access denied"));
        }

        if user_role != Role::SuperAdmin.as_str() && user_role != Role::ClientAdmin.as_str() {
            return Err(AppError::forbidden(
                "Access denied - Only Super Admin and Client Admin can create API keys",
            ));
        }

        let name = key_data.get("name").and_then(|v| v.as_str())
            .ok_or_else(|| AppError::bad_request("Name is required"))?;

        let description = key_data.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());
        let environment = key_data.get("environment").and_then(|v| v.as_str()).unwrap_or("production");
        let expires_at_minutes: i64 = key_data.get("expiresAt").and_then(|v| v.as_i64()).unwrap_or(24);

        let permissions_data = key_data.get("permissions").cloned().unwrap_or(serde_json::json!({}));
        let security_data = key_data.get("security").cloned().unwrap_or(serde_json::json!({}));

        let allowed_ips = security_data.get("allowedIPs")
            .and_then(|v| v.as_array())
            .filter(|arr| !arr.is_empty())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_else(|| vec!["0.0.0.0/0".to_string()]);

        let allowed_origins = security_data.get("allowedOrigins")
            .and_then(|v| v.as_array())
            .filter(|arr| !arr.is_empty())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_else(|| vec!["*".to_string()]);

        let client_oid = ObjectId::parse_str(client_id)
            .map_err(|_| AppError::bad_request("Invalid client ID"))?;
        let user_oid = ObjectId::parse_str(user_id)
            .map_err(|_| AppError::bad_request("Invalid user ID"))?;

        let api_key = ApiKey {
            id: None,
            key_id: Uuid::new_v4().to_string(),
            key_value: Self::generate_api_key(),
            client_id: client_oid,
            name: name.to_string(),
            description,
            environment: environment.to_string(),
            permissions: ApiKeyPermissions {
                can_ingest: permissions_data.get("canIngest").and_then(|v| v.as_bool()).unwrap_or(true),
                can_read_analytics: permissions_data.get("canReadAnalytics").and_then(|v| v.as_bool()).unwrap_or(false),
                allowed_services: permissions_data.get("allowedServices")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_default(),
            },
            security: ApiKeySecurity {
                allowed_i_ps: allowed_ips,
                allowed_origins,
                last_rotated: Some(Utc::now()),
                rotation_warning_days: security_data.get("rotationWarningDays")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(30) as u32,
            },
            is_active: true,
            created_by: Some(user_oid),
            expires_at: Some(Utc::now() + chrono::Duration::minutes(expires_at_minutes)),
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
        self.api_key_repository.find_by_client_id(client_id).await
    }

    /// Get client by API key value (used by API key validation middleware).
    pub async fn get_client_by_api_key(
        &self,
        api_key_value: &str,
    ) -> Result<Option<(Client, ApiKey)>, AppError> {
        let result = self.api_key_repository.find_by_key_value(api_key_value, false).await?;

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

    /// Validate API key access (authorization check).
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

        let api_key = self.api_key_repository.find_by_key_id(key_id).await?
            .ok_or_else(|| AppError::not_found("API key not found"))?;

        if api_key.client_id.to_hex() != client_id {
            return Err(AppError::bad_request("API key does not belong to this client"));
        }

        Ok(api_key)
    }

    /// Update an API key.
    pub async fn update_api_key(
        &self,
        client_id: &str,
        key_id: &str,
        update_data: serde_json::Value,
        user_role: &str,
        user_client_id: Option<&str>,
    ) -> Result<Option<ApiKey>, AppError> {
        let _existing = self.validate_api_key_access(client_id, key_id, user_role, user_client_id).await?;

        let mut updates = bson::Document::new();

        if let Some(name) = update_data.get("name").and_then(|v| v.as_str()) {
            updates.insert("name", name);
        }
        if let Some(desc) = update_data.get("description").and_then(|v| v.as_str()) {
            updates.insert("description", desc);
        }
        if let Some(env) = update_data.get("environment").and_then(|v| v.as_str()) {
            updates.insert("environment", env);
        }
        if let Some(perms) = update_data.get("permissions") {
            let can_ingest = perms.get("canIngest").and_then(|v| v.as_bool()).unwrap_or(true);
            let can_read = perms.get("canReadAnalytics").and_then(|v| v.as_bool()).unwrap_or(false);
            let services: Vec<String> = perms.get("allowedServices")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();

            updates.insert("permissions", doc! {
                "canIngest": can_ingest,
                "canReadAnalytics": can_read,
                "allowedServices": services,
            });
        }

        if let Some(expires) = update_data.get("expiresAt") {
            if let Some(minutes) = expires.as_i64() {
                updates.insert("expiresAt", Utc::now() + chrono::Duration::minutes(minutes));
            } else if let Some(date_str) = expires.as_str() {
                if let Ok(dt) = date_str.parse::<chrono::DateTime<Utc>>() {
                    updates.insert("expiresAt", dt);
                }
            }
        }

        self.api_key_repository.update_by_key_id(key_id, updates).await
    }

    /// Delete an API key.
    pub async fn delete_api_key(
        &self,
        client_id: &str,
        key_id: &str,
        user_role: &str,
        user_client_id: Option<&str>,
    ) -> Result<bool, AppError> {
        self.validate_api_key_access(client_id, key_id, user_role, user_client_id).await?;
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
        self.validate_api_key_access(client_id, key_id, user_role, user_client_id).await?;
        self.api_key_repository
            .update_by_key_id(key_id, doc! { "isActive": is_active })
            .await
    }

    /// Rotate an API key (generate new key value).
    pub async fn rotate_api_key(
        &self,
        client_id: &str,
        key_id: &str,
        options: serde_json::Value,
        user_role: &str,
        user_client_id: Option<&str>,
    ) -> Result<Option<ApiKey>, AppError> {
        self.validate_api_key_access(client_id, key_id, user_role, user_client_id).await?;

        let new_key_value = Self::generate_api_key();
        let mut updates = doc! {
            "keyValue": &new_key_value,
            "security.lastRotated": Utc::now(),
        };

        if let Some(expires) = options.get("expiresAt") {
            if let Some(minutes) = expires.as_i64() {
                updates.insert("expiresAt", Utc::now() + chrono::Duration::minutes(minutes));
            } else if let Some(date_str) = expires.as_str() {
                if let Ok(dt) = date_str.parse::<chrono::DateTime<Utc>>() {
                    updates.insert("expiresAt", dt);
                }
            }
        }

        self.api_key_repository.update_by_key_id(key_id, updates).await
    }

    /// Get API key details.
    pub async fn get_api_key_details(
        &self,
        client_id: &str,
        key_id: &str,
        user_role: &str,
        user_client_id: Option<&str>,
    ) -> Result<ApiKey, AppError> {
        self.validate_api_key_access(client_id, key_id, user_role, user_client_id).await
    }
}
