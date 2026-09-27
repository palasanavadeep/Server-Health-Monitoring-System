use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};

use crate::app_state::AppState;
use crate::domain::role::Role;
use crate::domain::tenant_config::TenantConfigUpdate;
use crate::dto::request::client::{
    CreateApiKeyRequest, CreateClientRequest, CreateClientUserRequest, RotateApiKeyRequest,
    UpdateApiKeyRequest,
};
use crate::error::app_error::AppError;
use crate::middleware::authenticate::AuthenticatedUser;
use crate::util::response::ResponseFormatter;

// ── Helper ─────────────────────────────────────────────────────────────────────

fn get_user(req: &HttpRequest) -> Option<AuthenticatedUser> {
    req.extensions().get::<AuthenticatedUser>().cloned()
}

macro_rules! require_user {
    ($req:expr) => {
        match get_user($req) {
            Some(u) => u,
            None => {
                return HttpResponse::Unauthorized()
                    .json(ResponseFormatter::unauthorized("Authentication required"))
            }
        }
    };
}

// ── Handlers ───────────────────────────────────────────────────────────────────

/// POST /api/admin/clients/onboard
pub async fn create_client(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<CreateClientRequest>,
) -> HttpResponse {
    let user = require_user!(&req);

    tracing::info!("User : {:?}", user);
    match state
        .auth_service
        .check_super_admin_permissions(&user.user_id)
        .await
    {
        Ok(true) => {}
        Ok(false) => {
            return HttpResponse::Forbidden().json(ResponseFormatter::forbidden("Access denied"))
        }
        Err(e) => return e.to_response(),
    }

    match state
        .client_service
        .create_client(body.into_inner(), &user.user_id)
        .await
    {
        Ok(client) => HttpResponse::Created().json(ResponseFormatter::created(
            client,
            "Client created successfully",
        )),
        Err(e) => e.to_response(),
    }
}

/// GET /api/admin/clients
pub async fn get_all_clients(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> HttpResponse {
    let user = require_user!(&req);

    match state.client_service.get_all_clients(&user.role).await {
        Ok(clients) => HttpResponse::Ok().json(ResponseFormatter::ok(
            clients,
            "Clients fetched successfully",
        )),
        Err(e) => e.to_response(),
    }
}

/// POST /api/admin/clients/{clientId}/users
pub async fn create_client_user(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<CreateClientUserRequest>,
) -> HttpResponse {
    let user = require_user!(&req);
    let client_id = path.into_inner();

    match state
        .client_service
        .create_client_user(
            &client_id,
            body.into_inner(),
            &user.role,
            user.client_id.as_deref(),
        )
        .await
    {
        Ok(user_resp) => HttpResponse::Created().json(ResponseFormatter::created(
            user_resp,
            "Client user created successfully",
        )),
        Err(e) => e.to_response(),
    }
}

/// GET /api/admin/clients/{clientId}/users
pub async fn get_client_users(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let user = require_user!(&req);
    let client_id = path.into_inner();

    match state
        .client_service
        .get_client_users(&client_id, &user.role, user.client_id.as_deref())
        .await
    {
        Ok(users) => {
            HttpResponse::Ok().json(ResponseFormatter::ok(users, "Client users fetched successfully"))
        }
        Err(e) => e.to_response(),
    }
}

/// GET /api/client/users (convenience endpoint for current logged-in user's client)
pub async fn get_current_client_users(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> HttpResponse {
    let user = require_user!(&req);
    let client_id = match user.client_id.as_deref() {
        Some(cid) => cid,
        None => {
            return AppError::bad_request(
                "Logged-in user is not associated with any client organization",
            )
            .to_response();
        }
    };

    match state
        .client_service
        .get_client_users(client_id, &user.role, user.client_id.as_deref())
        .await
    {
        Ok(users) => {
            HttpResponse::Ok().json(ResponseFormatter::ok(users, "Client users fetched successfully"))
        }
        Err(e) => e.to_response(),
    }
}

/// POST /api/admin/clients/{clientId}/api/keys
pub async fn create_api_key(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<CreateApiKeyRequest>,
) -> HttpResponse {
    let user = require_user!(&req);
    let client_id = path.into_inner();

    match state
        .client_service
        .create_api_key(
            &client_id,
            body.into_inner(),
            &user.role,
            user.client_id.as_deref(),
            &user.user_id,
        )
        .await
    {
        Ok(api_key) => HttpResponse::Created().json(ResponseFormatter::created(
            api_key,
            "API key created successfully",
        )),
        Err(e) => e.to_response(),
    }
}

/// GET /api/admin/clients/{clientId}/api/keys
pub async fn get_client_api_keys(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let user = require_user!(&req);
    let client_id = path.into_inner();

    match state
        .client_service
        .get_client_api_keys(&client_id, &user.role, user.client_id.as_deref())
        .await
    {
        Ok(keys) => {
            HttpResponse::Ok().json(ResponseFormatter::ok(keys, "API key fetched successfully"))
        }
        Err(e) => e.to_response(),
    }
}

#[derive(serde::Deserialize)]
pub struct ClientKeyPath {
    pub client_id: String,
    pub key_id: String,
}

/// PUT /api/admin/clients/{clientId}/api/keys/{keyId}
pub async fn update_api_key(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<ClientKeyPath>,
    body: web::Json<UpdateApiKeyRequest>,
) -> HttpResponse {
    let user = require_user!(&req);
    let params = path.into_inner();

    match state
        .client_service
        .update_api_key(
            &params.client_id,
            &params.key_id,
            body.into_inner(),
            &user.role,
            user.client_id.as_deref(),
        )
        .await
    {
        Ok(Some(updated)) => HttpResponse::Ok().json(ResponseFormatter::ok(
            updated,
            "API key updated successfully",
        )),
        Ok(None) => {
            HttpResponse::NotFound().json(ResponseFormatter::not_found("API key not found"))
        }
        Err(e) => e.to_response(),
    }
}

/// DELETE /api/admin/clients/{clientId}/api/keys/{keyId}
pub async fn delete_api_key(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<ClientKeyPath>,
) -> HttpResponse {
    let user = require_user!(&req);
    let params = path.into_inner();

    match state
        .client_service
        .delete_api_key(
            &params.client_id,
            &params.key_id,
            &user.role,
            user.client_id.as_deref(),
        )
        .await
    {
        Ok(_) => HttpResponse::Ok().json(ResponseFormatter::ok(
            serde_json::json!({}),
            "API key deleted successfully",
        )),
        Err(e) => e.to_response(),
    }
}

/// PATCH /api/admin/clients/{clientId}/api/keys/{keyId}/deactivate
pub async fn deactivate_api_key(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<ClientKeyPath>,
) -> HttpResponse {
    let user = require_user!(&req);
    let params = path.into_inner();

    match state
        .client_service
        .toggle_api_key_status(
            &params.client_id,
            &params.key_id,
            false,
            &user.role,
            user.client_id.as_deref(),
        )
        .await
    {
        Ok(Some(updated)) => HttpResponse::Ok().json(ResponseFormatter::ok(
            updated,
            "API key deactivated successfully",
        )),
        Ok(None) => {
            HttpResponse::NotFound().json(ResponseFormatter::not_found("API key not found"))
        }
        Err(e) => e.to_response(),
    }
}

/// PATCH /api/admin/clients/{clientId}/api/keys/{keyId}/activate
pub async fn activate_api_key(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<ClientKeyPath>,
) -> HttpResponse {
    let user = require_user!(&req);
    let params = path.into_inner();

    match state
        .client_service
        .toggle_api_key_status(
            &params.client_id,
            &params.key_id,
            true,
            &user.role,
            user.client_id.as_deref(),
        )
        .await
    {
        Ok(Some(updated)) => HttpResponse::Ok().json(ResponseFormatter::ok(
            updated,
            "API key activated successfully",
        )),
        Ok(None) => {
            HttpResponse::NotFound().json(ResponseFormatter::not_found("API key not found"))
        }
        Err(e) => e.to_response(),
    }
}

/// POST /api/admin/clients/{clientId}/api/keys/{keyId}/rotate
pub async fn rotate_api_key(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<ClientKeyPath>,
    body: web::Json<RotateApiKeyRequest>,
) -> HttpResponse {
    let user = require_user!(&req);
    let params = path.into_inner();

    match state
        .client_service
        .rotate_api_key(
            &params.client_id,
            &params.key_id,
            body.into_inner(),
            &user.role,
            user.client_id.as_deref(),
        )
        .await
    {
        Ok(Some(updated)) => HttpResponse::Ok().json(ResponseFormatter::ok(
            updated,
            "API key rotated successfully",
        )),
        Ok(None) => {
            HttpResponse::NotFound().json(ResponseFormatter::not_found("API key not found"))
        }
        Err(e) => e.to_response(),
    }
}

/// GET /api/admin/clients/{clientId}/api/keys/{keyId}
pub async fn get_api_key(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<ClientKeyPath>,
) -> HttpResponse {
    let user = require_user!(&req);
    let params = path.into_inner();

    match state
        .client_service
        .get_api_key_details(
            &params.client_id,
            &params.key_id,
            &user.role,
            user.client_id.as_deref(),
        )
        .await
    {
        Ok(api_key) => HttpResponse::Ok().json(ResponseFormatter::ok(
            api_key,
            "API key details retrieved successfully",
        )),
        Err(e) => e.to_response(),
    }
}

#[derive(serde::Deserialize)]
pub struct ClientIdParam {
    #[serde(alias = "clientId")]
    pub client_id: String,
}

/// GET /api/admin/clients/{clientId}/config
pub async fn get_client_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<ClientIdParam>,
) -> HttpResponse {
    let user = require_user!(&req);
    let params = path.into_inner();

    let is_super_admin = match state.auth_service.check_super_admin_permissions(&user.user_id).await {
        Ok(b) => b,
        Err(e) => return e.to_response(),
    };

    if !is_super_admin {
        match &user.client_id {
            Some(cid) if cid == &params.client_id => {}
            _ => return HttpResponse::Forbidden().json(ResponseFormatter::forbidden("Access denied")),
        }
    }

    match state.tenant_config_service.get_config(&params.client_id).await {
        Ok(config) => HttpResponse::Ok().json(ResponseFormatter::ok(
            config.as_ref(),
            "Tenant config retrieved successfully",
        )),
        Err(e) => e.to_response(),
    }
}

/// PUT /api/admin/clients/{clientId}/config
pub async fn update_client_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<ClientIdParam>,
    body: web::Json<TenantConfigUpdate>,
) -> HttpResponse {
    let user = require_user!(&req);
    let params = path.into_inner();

    let is_super_admin = match state.auth_service.check_super_admin_permissions(&user.user_id).await {
        Ok(b) => b,
        Err(e) => return e.to_response(),
    };

    if !is_super_admin {
        if user.role != Role::ClientAdmin.as_str() {
            return HttpResponse::Forbidden().json(ResponseFormatter::forbidden(
                "Only super admins and client admins can update configuration",
            ));
        }
        match &user.client_id {
            Some(cid) if cid == &params.client_id => {}
            _ => {
                return HttpResponse::Forbidden().json(ResponseFormatter::forbidden(
                    "Access denied: You can only update your own client configuration",
                ))
            }
        }
    }

    match state.tenant_config_service.update_config(&params.client_id, &body.into_inner()).await {
        Ok(()) => {
            match state.tenant_config_service.get_config(&params.client_id).await {
                Ok(cfg) => HttpResponse::Ok().json(ResponseFormatter::ok(
                    cfg.as_ref(),
                    "Tenant config updated successfully",
                )),
                Err(e) => e.to_response(),
            }
        }
        Err(e) => e.to_response(),
    }
}

/// GET /api/client/config
pub async fn get_current_client_config(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> HttpResponse {
    let user = require_user!(&req);
    let client_id = match &user.client_id {
        Some(cid) => cid,
        None => return HttpResponse::Forbidden().json(ResponseFormatter::forbidden("No client associated with user")),
    };

    match state.tenant_config_service.get_config(client_id).await {
        Ok(config) => HttpResponse::Ok().json(ResponseFormatter::ok(
            config.as_ref(),
            "Tenant config retrieved successfully",
        )),
        Err(e) => e.to_response(),
    }
}

/// PUT /api/client/config
pub async fn update_current_client_config(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<TenantConfigUpdate>,
) -> HttpResponse {
    let user = require_user!(&req);

    if user.role != Role::SuperAdmin.as_str() && user.role != Role::ClientAdmin.as_str() {
        return HttpResponse::Forbidden().json(ResponseFormatter::forbidden(
            "Only super admins and client admins can update configuration",
        ));
    }

    let client_id = match &user.client_id {
        Some(cid) => cid,
        None => return HttpResponse::Forbidden().json(ResponseFormatter::forbidden("No client associated with user")),
    };

    match state.tenant_config_service.update_config(client_id, &body.into_inner()).await {
        Ok(()) => {
            match state.tenant_config_service.get_config(client_id).await {
                Ok(cfg) => HttpResponse::Ok().json(ResponseFormatter::ok(
                    cfg.as_ref(),
                    "Tenant config updated successfully",
                )),
                Err(e) => e.to_response(),
            }
        }
        Err(e) => e.to_response(),
    }
}

/// GET /api/admin/histogram-profiles
pub async fn list_histogram_profiles(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> HttpResponse {
    let _user = require_user!(&req);

    match state.tenant_config_service.list_profiles().await {
        Ok(profiles) => HttpResponse::Ok().json(ResponseFormatter::ok(
            profiles,
            "Histogram profiles retrieved successfully",
        )),
        Err(e) => e.to_response(),
    }
}

