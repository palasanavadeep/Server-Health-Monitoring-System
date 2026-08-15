use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};

use crate::app_state::AppState;
use crate::error::app_error::AppError;
use crate::middleware::authenticate::AuthenticatedUser;
use crate::util::response::ResponseFormatter;

// ── Query params ───────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct AnalyticsQuery {
    #[serde(rename = "startTime")]
    pub start_time: Option<String>,
    #[serde(rename = "endTime")]
    pub end_time: Option<String>,
    #[serde(rename = "clientId")]
    pub client_id: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct ApiMetricsQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    #[serde(rename = "clientId")]
    pub client_id: Option<String>,
}

// ── Private helpers ────────────────────────────────────────────────────────────

fn get_user(req: &HttpRequest) -> Option<AuthenticatedUser> {
    req.extensions().get::<AuthenticatedUser>().cloned()
}

fn parse_time_range(
    start: Option<&str>,
    end: Option<&str>,
) -> Result<(Option<i64>, Option<i64>), AppError> {
    let parse = |v: &str| -> Result<i64, AppError> {
        if v.chars().all(|c| c.is_ascii_digit()) {
            v.parse::<i64>()
                .map_err(|_| AppError::bad_request("Invalid time format"))
        } else {
            chrono::DateTime::parse_from_rfc3339(v)
                .map(|d| d.timestamp_millis())
                .or_else(|_| {
                    v.parse::<chrono::NaiveDateTime>()
                        .map(|d| d.and_utc().timestamp_millis())
                })
                .map_err(|_| AppError::bad_request("Invalid time format"))
        }
    };

    let s = match start {
        Some(v) if !v.is_empty() => Some(parse(v)?),
        _ => None,
    };
    let e = match end {
        Some(v) if !v.is_empty() => Some(parse(v)?),
        _ => None,
    };

    if let (Some(s), Some(e)) = (s, e) {
        if s > e {
            return Err(AppError::bad_request("Invalid time range: start > end"));
        }
    }
    Ok((s, e))
}

async fn resolve_client_id(
    state: &web::Data<AppState>,
    user: &AuthenticatedUser,
    query_client_id: Option<&str>,
) -> Result<Option<String>, AppError> {
    let is_super_admin = state
        .auth_service
        .check_super_admin_permissions(&user.user_id)
        .await?;

    if !is_super_admin {
        let profile = state.auth_service.get_profile(&user.user_id).await?;
        if !profile
            .permissions
            .as_ref()
            .map(|p| p.can_view_analytics)
            .unwrap_or(false)
        {
            return Err(AppError::forbidden(
                "Insufficient permissions to view analytics",
            ));
        }
    }

    if is_super_admin {
        if let Some(cid) = query_client_id {
            if cid.len() != 24 || !cid.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(AppError::bad_request("Invalid clientId format"));
            }
            Ok(Some(cid.to_string()))
        } else {
            Ok(None)
        }
    } else {
        let cid = user
            .client_id
            .as_ref()
            .ok_or_else(|| AppError::forbidden("Access denied - no client association"))?;
        Ok(Some(cid.clone()))
    }
}

// ── Handlers ───────────────────────────────────────────────────────────────────

/// GET /api/analytics/stats
pub async fn get_stats(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<AnalyticsQuery>,
) -> HttpResponse {
    let user = match get_user(&req) {
        Some(u) => u,
        None => {
            return HttpResponse::Unauthorized().json(ResponseFormatter::error(
                "Authentication required",
                401,
                None,
            ))
        }
    };

    let client_id = match resolve_client_id(&state, &user, query.client_id.as_deref()).await {
        Ok(c) => c,
        Err(e) => return e.to_response(),
    };

    let (start, end) =
        match parse_time_range(query.start_time.as_deref(), query.end_time.as_deref()) {
            Ok(r) => r,
            Err(e) => return e.to_response(),
        };

    match state
        .analytics_service
        .get_overall_stats(client_id.as_deref(), start, end)
        .await
    {
        Ok(stats) => HttpResponse::Ok().json(ResponseFormatter::success(
            serde_json::json!(stats),
            "Statistics retrieved successfully",
            200,
        )),
        Err(e) => e.to_response(),
    }
}

/// GET /api/analytics/dashboard
pub async fn get_dashboard(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<AnalyticsQuery>,
) -> HttpResponse {
    let user = match get_user(&req) {
        Some(u) => u,
        None => {
            return HttpResponse::Unauthorized().json(ResponseFormatter::error(
                "Authentication required",
                401,
                None,
            ))
        }
    };

    let client_id = match resolve_client_id(&state, &user, query.client_id.as_deref()).await {
        Ok(c) => c,
        Err(e) => return e.to_response(),
    };

    let (start, end) =
        match parse_time_range(query.start_time.as_deref(), query.end_time.as_deref()) {
            Ok(r) => r,
            Err(e) => return e.to_response(),
        };

    // Dashboard data fetched in parallel inside the service — typed DashboardData returned
    let dashboard = state
        .analytics_service
        .get_dashboard(client_id.as_deref(), start, end)
        .await;

    HttpResponse::Ok().json(ResponseFormatter::success(
        serde_json::json!(dashboard),
        "Dashboard data retrieved successfully",
        200,
    ))
}

/// GET /api/analytics/apis
pub async fn get_apis_metrics(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<ApiMetricsQuery>,
) -> HttpResponse {
    let user = match get_user(&req) {
        Some(u) => u,
        None => {
            return HttpResponse::Unauthorized().json(ResponseFormatter::error(
                "Authentication required",
                401,
                None,
            ))
        }
    };

    let client_id = match resolve_client_id(&state, &user, query.client_id.as_deref()).await {
        Ok(c) => c,
        Err(e) => return e.to_response(),
    };

    let client_id = match client_id {
        Some(id) => id,
        None => {
            return HttpResponse::BadRequest()
                .json(ResponseFormatter::error("clientId is required", 400, None))
        }
    };

    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(10);

    match state
        .analytics_service
        .get_client_apis_metrics(&client_id, page, limit)
        .await
    {
        Ok(metrics) => HttpResponse::Ok().json(ResponseFormatter::success(
            serde_json::json!(metrics),
            "API metrics retrieved successfully",
            200,
        )),
        Err(e) => e.to_response(),
    }
}
