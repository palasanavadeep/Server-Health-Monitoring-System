use actix_web::{
    cookie::{time::Duration as CookieDuration, Cookie, SameSite},
    web, HttpMessage, HttpRequest, HttpResponse,
};
use validator::Validate;

use crate::app_state::AppState;
use crate::domain::role::Role;
use crate::dto::request::auth::{
    LoginRequest, OnboardSuperAdminRequest, RegisterRequest, UpdateProfileRequest,
};
use crate::middleware::authenticate::AuthenticatedUser;
use crate::util::response::ResponseFormatter;

/// POST /api/auth/onboard-super-admin
pub async fn onboard_super_admin(
    state: web::Data<AppState>,
    body: web::Json<OnboardSuperAdminRequest>,
) -> HttpResponse {
    if let Err(e) = body.validate() {
        return HttpResponse::BadRequest().json(ResponseFormatter::validation_error(Some(
            serde_json::to_value(e.to_string()).unwrap(),
        )));
    }

    match state
        .auth_service
        .onboard_super_admin(&body.username, &body.email, &body.password)
        .await
    {
        Ok(user) => HttpResponse::Created().json(ResponseFormatter::created(
            user,
            "Super admin registered successfully",
        )),
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
        return HttpResponse::BadRequest().json(ResponseFormatter::validation_error(Some(
            serde_json::to_value(e.to_string()).unwrap(),
        )));
    }

    let user = match req.extensions().get::<AuthenticatedUser>() {
        Some(u) => u.clone(),
        None => {
            return HttpResponse::Unauthorized()
                .json(ResponseFormatter::unauthorized("Authentication required"));
        }
    };

    if user.role != Role::SuperAdmin.as_str() {
        return HttpResponse::Forbidden().json(ResponseFormatter::forbidden("Access denied"));
    }

    let role = body.role.as_deref().unwrap_or(Role::ClientViewer.as_str());

    match state
        .auth_service
        .register(&body.username, &body.email, &body.password, role)
        .await
    {
        Ok(user) => HttpResponse::Created().json(ResponseFormatter::created(
            user,
            "User registered successfully",
        )),
        Err(e) => e.to_response(),
    }
}

/// POST /api/auth/login
pub async fn login(state: web::Data<AppState>, body: web::Json<LoginRequest>) -> HttpResponse {
    if let Err(e) = body.validate() {
        return HttpResponse::BadRequest().json(ResponseFormatter::validation_error(Some(
            serde_json::to_value(e.to_string()).unwrap(),
        )));
    }

    match state.auth_service.login(&body.email, &body.password).await {
        Ok((user, token)) => {
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
                .json(ResponseFormatter::ok(user, "Login successful"))
        }
        Err(e) => e.to_response(),
    }
}

/// GET /api/auth/profile
pub async fn get_profile(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let user = match req.extensions().get::<AuthenticatedUser>() {
        Some(u) => u.clone(),
        None => {
            return HttpResponse::Unauthorized()
                .json(ResponseFormatter::unauthorized("Authentication required"));
        }
    };

    match state.auth_service.get_profile(&user.user_id).await {
        Ok(profile) => HttpResponse::Ok().json(ResponseFormatter::ok(
            profile,
            "Profile retrieved successfully",
        )),
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
            return HttpResponse::Unauthorized()
                .json(ResponseFormatter::unauthorized("Authentication required"));
        }
    };

    let updates = crate::domain::updates::UserProfileUpdate {
        username: body.username.clone(),
        email: body.email.clone(),
    };

    match state
        .auth_service
        .update_profile(&user.user_id, updates)
        .await
    {
        Ok(profile) => HttpResponse::Ok().json(ResponseFormatter::ok(
            profile,
            "Profile updated successfully",
        )),
        Err(e) => e.to_response(),
    }
}

/// PATCH /api/auth/users/{userId}/deactivate
pub async fn deactivate_user(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let caller = match req.extensions().get::<AuthenticatedUser>() {
        Some(u) => u.clone(),
        None => {
            return HttpResponse::Unauthorized()
                .json(ResponseFormatter::unauthorized("Authentication required"));
        }
    };
    let target_user_id = path.into_inner();

    match state
        .auth_service
        .deactivate_user(
            &target_user_id,
            &caller.user_id,
            &caller.role,
            caller.client_id.as_deref(),
        )
        .await
    {
        Ok(user) => {
            HttpResponse::Ok().json(ResponseFormatter::ok(user, "User deactivated successfully"))
        }
        Err(e) => e.to_response(),
    }
}

/// PATCH /api/auth/users/{userId}/activate
pub async fn activate_user(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let caller = match req.extensions().get::<AuthenticatedUser>() {
        Some(u) => u.clone(),
        None => {
            return HttpResponse::Unauthorized()
                .json(ResponseFormatter::unauthorized("Authentication required"));
        }
    };
    let target_user_id = path.into_inner();

    match state
        .auth_service
        .activate_user(
            &target_user_id,
            &caller.user_id,
            &caller.role,
            caller.client_id.as_deref(),
        )
        .await
    {
        Ok(user) => {
            HttpResponse::Ok().json(ResponseFormatter::ok(user, "User activated successfully"))
        }
        Err(e) => e.to_response(),
    }
}

/// GET /api/auth/logout
pub async fn logout() -> HttpResponse {
    let cookie = Cookie::build("authToken", "")
        .path("/")
        .http_only(true)
        .max_age(CookieDuration::ZERO)
        .same_site(SameSite::Lax)
        .finish();

    HttpResponse::Ok()
        .cookie(cookie)
        .json(ResponseFormatter::ok(
            serde_json::json!({}),
            "Logout successful",
        ))
}
