use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};

use crate::app_state::AppState;
use crate::middleware::authenticate::AuthenticatedUser;
use crate::util::response::ResponseFormatter;

/// POST /api/admin/clients/onboard
pub async fn create_client(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let user = match req.extensions().get::<AuthenticatedUser>() {
        Some(u) => u.clone(),
        None => return HttpResponse::Unauthorized().json(ResponseFormatter::error("Authentication required", 401, None)),
    };

    match state.auth_service.check_super_admin_permissions(&user.user_id).await {
        Ok(true) => {}
        Ok(false) => return HttpResponse::Forbidden().json(ResponseFormatter::error("Access denied", 403, None)),
        Err(e) => return e.to_response(),
    }

    match state.client_service.create_client(body.into_inner(), &user.user_id).await {
        Ok(client) => {
            let value = serde_json::to_value(&client).unwrap_or_default();
            HttpResponse::Created().json(ResponseFormatter::success(value, "Client created successfully", 201))
        }
        Err(e) => e.to_response(),
    }
}

/// POST /api/admin/clients/{clientId}/users
pub async fn create_client_user(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let user = match req.extensions().get::<AuthenticatedUser>() {
        Some(u) => u.clone(),
        None => return HttpResponse::Unauthorized().json(ResponseFormatter::error("Authentication required", 401, None)),
    };

    let client_id = path.into_inner();

    match state.client_service.create_client_user(
        &client_id,
        body.into_inner(),
        &user.role,
        user.client_id.as_deref(),
    ).await {
        Ok(user_resp) => {
            let value = serde_json::to_value(&user_resp).unwrap_or_default();
            HttpResponse::Created().json(ResponseFormatter::success(value, "Client user created successfully", 201))
        }
        Err(e) => e.to_response(),
    }
}

/// POST /api/admin/clients/{clientId}/api/keys
pub async fn create_api_key(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let user = match req.extensions().get::<AuthenticatedUser>() {
        Some(u) => u.clone(),
        None => return HttpResponse::Unauthorized().json(ResponseFormatter::error("Authentication required", 401, None)),
    };

    let client_id = path.into_inner();

    match state.client_service.create_api_key(
        &client_id,
        body.into_inner(),
        &user.role,
        user.client_id.as_deref(),
        &user.user_id,
    ).await {
        Ok(api_key) => {
            let value = serde_json::to_value(&api_key).unwrap_or_default();
            HttpResponse::Created().json(ResponseFormatter::success(value, "API key created successfully", 201))
        }
        Err(e) => e.to_response(),
    }
}

/// GET /api/admin/clients/{clientId}/api/keys
pub async fn get_client_api_keys(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let user = match req.extensions().get::<AuthenticatedUser>() {
        Some(u) => u.clone(),
        None => return HttpResponse::Unauthorized().json(ResponseFormatter::error("Authentication required", 401, None)),
    };

    let client_id = path.into_inner();

    match state.client_service.get_client_api_keys(&client_id, &user.role, user.client_id.as_deref()).await {
        Ok(keys) => {
            let value = serde_json::to_value(&keys).unwrap_or_default();
            HttpResponse::Ok().json(ResponseFormatter::success(value, "API key fetched successfully", 200))
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
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let user = match req.extensions().get::<AuthenticatedUser>() {
        Some(u) => u.clone(),
        None => return HttpResponse::Unauthorized().json(ResponseFormatter::error("Authentication required", 401, None)),
    };
    let params = path.into_inner();
    match state.client_service.update_api_key(
        &params.client_id, &params.key_id, body.into_inner(), &user.role, user.client_id.as_deref(),
    ).await {
        Ok(Some(updated)) => {
            let value = serde_json::to_value(&updated).unwrap_or_default();
            HttpResponse::Ok().json(ResponseFormatter::success(value, "API key updated successfully", 200))
        }
        Ok(None) => HttpResponse::NotFound().json(ResponseFormatter::error("API key not found", 404, None)),
        Err(e) => e.to_response(),
    }
}

/// DELETE /api/admin/clients/{clientId}/api/keys/{keyId}
pub async fn delete_api_key(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<ClientKeyPath>,
) -> HttpResponse {
    let user = match req.extensions().get::<AuthenticatedUser>() {
        Some(u) => u.clone(),
        None => return HttpResponse::Unauthorized().json(ResponseFormatter::error("Authentication required", 401, None)),
    };
    let params = path.into_inner();
    match state.client_service.delete_api_key(
        &params.client_id, &params.key_id, &user.role, user.client_id.as_deref(),
    ).await {
        Ok(_) => HttpResponse::Ok().json(ResponseFormatter::success(serde_json::json!({}), "API key deleted successfully", 200)),
        Err(e) => e.to_response(),
    }
}

/// PATCH /api/admin/clients/{clientId}/api/keys/{keyId}/deactivate
pub async fn deactivate_api_key(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<ClientKeyPath>,
) -> HttpResponse {
    let user = match req.extensions().get::<AuthenticatedUser>() {
        Some(u) => u.clone(),
        None => return HttpResponse::Unauthorized().json(ResponseFormatter::error("Authentication required", 401, None)),
    };
    let params = path.into_inner();
    match state.client_service.toggle_api_key_status(
        &params.client_id, &params.key_id, false, &user.role, user.client_id.as_deref(),
    ).await {
        Ok(Some(updated)) => {
            let value = serde_json::to_value(&updated).unwrap_or_default();
            HttpResponse::Ok().json(ResponseFormatter::success(value, "API key deactivated successfully", 200))
        }
        Ok(None) => HttpResponse::NotFound().json(ResponseFormatter::error("API key not found", 404, None)),
        Err(e) => e.to_response(),
    }
}

/// PATCH /api/admin/clients/{clientId}/api/keys/{keyId}/activate
pub async fn activate_api_key(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<ClientKeyPath>,
) -> HttpResponse {
    let user = match req.extensions().get::<AuthenticatedUser>() {
        Some(u) => u.clone(),
        None => return HttpResponse::Unauthorized().json(ResponseFormatter::error("Authentication required", 401, None)),
    };
    let params = path.into_inner();
    match state.client_service.toggle_api_key_status(
        &params.client_id, &params.key_id, true, &user.role, user.client_id.as_deref(),
    ).await {
        Ok(Some(updated)) => {
            let value = serde_json::to_value(&updated).unwrap_or_default();
            HttpResponse::Ok().json(ResponseFormatter::success(value, "API key activated successfully", 200))
        }
        Ok(None) => HttpResponse::NotFound().json(ResponseFormatter::error("API key not found", 404, None)),
        Err(e) => e.to_response(),
    }
}

/// POST /api/admin/clients/{clientId}/api/keys/{keyId}/rotate
pub async fn rotate_api_key(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<ClientKeyPath>,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let user = match req.extensions().get::<AuthenticatedUser>() {
        Some(u) => u.clone(),
        None => return HttpResponse::Unauthorized().json(ResponseFormatter::error("Authentication required", 401, None)),
    };
    let params = path.into_inner();
    match state.client_service.rotate_api_key(
        &params.client_id, &params.key_id, body.into_inner(), &user.role, user.client_id.as_deref(),
    ).await {
        Ok(Some(updated)) => {
            let value = serde_json::to_value(&updated).unwrap_or_default();
            HttpResponse::Ok().json(ResponseFormatter::success(value, "API key rotated successfully", 200))
        }
        Ok(None) => HttpResponse::NotFound().json(ResponseFormatter::error("API key not found", 404, None)),
        Err(e) => e.to_response(),
    }
}

/// GET /api/admin/clients/{clientId}/api/keys/{keyId}
pub async fn get_api_key(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<ClientKeyPath>,
) -> HttpResponse {
    let user = match req.extensions().get::<AuthenticatedUser>() {
        Some(u) => u.clone(),
        None => return HttpResponse::Unauthorized().json(ResponseFormatter::error("Authentication required", 401, None)),
    };
    let params = path.into_inner();
    match state.client_service.get_api_key_details(
        &params.client_id, &params.key_id, &user.role, user.client_id.as_deref(),
    ).await {
        Ok(api_key) => {
            let value = serde_json::to_value(&api_key).unwrap_or_default();
            HttpResponse::Ok().json(ResponseFormatter::success(value, "API key details retrieved successfully", 200))
        }
        Err(e) => e.to_response(),
    }
}
