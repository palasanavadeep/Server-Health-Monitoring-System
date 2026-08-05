use actix_web::{web, HttpRequest, HttpResponse, HttpMessage, cookie::{Cookie, SameSite, time::Duration as CookieDuration}};
use validator::Validate;

use crate::app_state::AppState;
use crate::middleware::authenticate::AuthenticatedUser;
use crate::utils::response_formatter::ResponseFormatter;
use super::validation::{OnboardSuperAdminRequest, RegisterRequest, LoginRequest, UpdateProfileRequest};

/// POST /api/auth/onboard-super-admin
pub async fn onboard_super_admin(
    state: web::Data<AppState>,
    body: web::Json<OnboardSuperAdminRequest>,
) -> HttpResponse {
    if let Err(e) = body.validate() {
        return HttpResponse::BadRequest().json(
            ResponseFormatter::validation_error(Some(serde_json::to_value(e.to_string()).unwrap())),
        );
    }

    match state
        .auth_service
        .onboard_super_admin(&body.username, &body.email, &body.password)
        .await
    {
        Ok(user) => {
            let user_value = serde_json::to_value(&user).unwrap_or_default();
            HttpResponse::Created().json(
                ResponseFormatter::success(user_value, "Super admin registered successfully", 201),
            )
        }
        Err(e) => e.to_response(),
    }
}

/// POST /api/auth/register
pub async fn register(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<RegisterRequest>,
) -> HttpResponse {
    if let Err(e) = body.validate() {
        return HttpResponse::BadRequest().json(
            ResponseFormatter::validation_error(Some(serde_json::to_value(e.to_string()).unwrap())),
        );
    }

    // Check that requester is super_admin
    let user = match req.extensions().get::<AuthenticatedUser>() {
        Some(u) => u.clone(),
        None => {
            return HttpResponse::Unauthorized().json(
                ResponseFormatter::error("Authentication required", 401, None),
            );
        }
    };

    if user.role != crate::constants::roles::ApplicationRoles::SUPER_ADMIN {
        return HttpResponse::Forbidden().json(
            ResponseFormatter::error("Access denied", 403, None),
        );
    }

    let role = body.role.as_deref().unwrap_or("client_viewer");

    match state
        .auth_service
        .register(&body.username, &body.email, &body.password, role)
        .await
    {
        Ok(user) => {
            let user_value = serde_json::to_value(&user).unwrap_or_default();
            HttpResponse::Created().json(
                ResponseFormatter::success(user_value, "User registered successfully", 201),
            )
        }
        Err(e) => e.to_response(),
    }
}

/// POST /api/auth/login
pub async fn login(
    state: web::Data<AppState>,
    body: web::Json<LoginRequest>,
) -> HttpResponse {
    if let Err(e) = body.validate() {
        return HttpResponse::BadRequest().json(
            ResponseFormatter::validation_error(Some(serde_json::to_value(e.to_string()).unwrap())),
        );
    }

    match state
        .auth_service
        .login(&body.email, &body.password)
        .await
    {
        Ok((user, token)) => {
            let user_value = serde_json::to_value(&user).unwrap_or_default();

            // Set authToken cookie (mirrors Node.js: httpOnly, secure in production, SameSite=Lax)
            let cookie = Cookie::build("authToken", token)
                .path("/")
                .http_only(state.config.cookie.http_only)
                .secure(state.config.cookie.secure)
                .same_site(SameSite::Lax)
                .max_age(CookieDuration::milliseconds(
                    state.config.cookie.expires_in_ms as i64,
                ))
                .finish();

            HttpResponse::Ok()
                .cookie(cookie)
                .json(ResponseFormatter::success(
                    user_value,
                    "Login successful",
                    200,
                ))
        }
        Err(e) => e.to_response(),
    }
}

/// GET /api/auth/profile
pub async fn get_profile(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> HttpResponse {
    let user = match req.extensions().get::<AuthenticatedUser>() {
        Some(u) => u.clone(),
        None => {
            return HttpResponse::Unauthorized().json(
                ResponseFormatter::error("Authentication required", 401, None),
            );
        }
    };

    match state.auth_service.get_profile(&user.user_id).await {
        Ok(profile) => {
            let value = serde_json::to_value(&profile).unwrap_or_default();
            HttpResponse::Ok().json(ResponseFormatter::success(
                value,
                "Profile retrieved successfully",
                200,
            ))
        }
        Err(e) => e.to_response(),
    }
}

/// PUT /api/auth/profile
pub async fn update_profile(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<UpdateProfileRequest>,
) -> HttpResponse {
    let user = match req.extensions().get::<AuthenticatedUser>() {
        Some(u) => u.clone(),
        None => {
            return HttpResponse::Unauthorized().json(
                ResponseFormatter::error("Authentication required", 401, None),
            );
        }
    };

    let updates = serde_json::json!({
        "username": body.username,
        "email": body.email,
    });

    match state
        .auth_service
        .update_profile(&user.user_id, updates)
        .await
    {
        Ok(profile) => {
            let value = serde_json::to_value(&profile).unwrap_or_default();
            HttpResponse::Ok().json(ResponseFormatter::success(
                value,
                "Profile updated successfully",
                200,
            ))
        }
        Err(e) => e.to_response(),
    }
}

/// PATCH /api/auth/users/:userId/deactivate
pub async fn deactivate_user(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let user_id = path.into_inner();

    match state.auth_service.deactivate_user(&user_id).await {
        Ok(user) => {
            let value = serde_json::to_value(&user).unwrap_or_default();
            HttpResponse::Ok().json(ResponseFormatter::success(
                value,
                "User deactivated successfully",
                200,
            ))
        }
        Err(e) => e.to_response(),
    }
}

/// GET /api/auth/logout
pub async fn logout() -> HttpResponse {
    // Clear the authToken cookie
    let cookie = Cookie::build("authToken", "")
        .path("/")
        .http_only(true)
        .max_age(CookieDuration::ZERO)
        .same_site(SameSite::Lax)
        .finish();

    HttpResponse::Ok()
        .cookie(cookie)
        .json(ResponseFormatter::success(
            serde_json::json!({}),
            "Logout successful",
            200,
        ))
}
