use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};

use crate::app_state::AppState;
use crate::error::app_error::AppError;
use crate::middleware::authenticate::AuthenticatedUser;
use crate::util::response::ResponseFormatter;

/// GET /api/analytics/stats
pub async fn get_stats(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<AnalyticsQuery>,
) -> HttpResponse {
    let user = match get_user(&req) {
        Some(u) => u,
        None => return HttpResponse::Unauthorized().json(ResponseFormatter::error("Authentication required", 401, None)),
    };

    match ensure_and_resolve(&state, &user, query.client_id.as_deref()).await {
        Ok(final_client_id) => {
            let time_range = match validate_time_range(query.start_time.as_deref(), query.end_time.as_deref()) {
                Ok(r) => r,
                Err(e) => return e.to_response(),
            };

            match state.analytics_service.get_overall_stats(
                final_client_id.as_deref(), time_range.0, time_range.1,
            ).await {
                Ok(stats) => HttpResponse::Ok().json(
                    ResponseFormatter::success(stats, "Statistics retrieved successfully", 200),
                ),
                Err(e) => e.to_response(),
            }
        }
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
        None => return HttpResponse::Unauthorized().json(ResponseFormatter::error("Authentication required", 401, None)),
    };

    match ensure_and_resolve(&state, &user, query.client_id.as_deref()).await {
        Ok(final_client_id) => {
            let time_range = match validate_time_range(query.start_time.as_deref(), query.end_time.as_deref()) {
                Ok(r) => r,
                Err(e) => return e.to_response(),
            };

            // Parallel fetch (mirrors Promise.allSettled pattern from Node.js)
            let stats_fut = state.analytics_service.get_overall_stats(
                final_client_id.as_deref(), time_range.0, time_range.1,
            );
            let top_fut = state.analytics_service.get_top_endpoints(
                final_client_id.as_deref(), 5, time_range.0,
            );
            let ts_fut = state.analytics_service.get_time_series(
                final_client_id.as_deref(), time_range.0, time_range.1, 24,
            );

            let (stats_res, top_res, ts_res) = tokio::join!(stats_fut, top_fut, ts_fut);

            let stats = stats_res.ok();
            let top_endpoints = top_res.ok();
            let recent_activity = ts_res.ok();

            let dashboard = serde_json::json!({
                "stats": stats,
                "topEndpoints": top_endpoints,
                "recentActitivy": recent_activity, // preserve original typo from Node.js
            });

            HttpResponse::Ok().json(
                ResponseFormatter::success(dashboard, "Dashboard data retrieved successfully", 200),
            )
        }
        Err(e) => e.to_response(),
    }
}

/// GET /api/analytics/apis
pub async fn get_apis_metrics(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<ApiMetricsQuery>,
) -> HttpResponse {
    let user = match get_user(&req) {
        Some(u) => u,
        None => return HttpResponse::Unauthorized().json(ResponseFormatter::error("Authentication required", 401, None)),
    };

    match ensure_and_resolve(&state, &user, query.client_id.as_deref()).await {
        Ok(final_client_id) => {
            let client_id = match final_client_id {
                Some(id) => id,
                None => return HttpResponse::BadRequest().json(
                    ResponseFormatter::error("clientId is required", 400, None),
                ),
            };

            let page = query.page.unwrap_or(1);
            let limit = query.limit.unwrap_or(10);

            match state.analytics_service.get_client_apis_metrics(&client_id, page, limit).await {
                Ok(metrics) => HttpResponse::Ok().json(
                    ResponseFormatter::success(metrics, "API metrics retrieved successfully", 200),
                ),
                Err(e) => e.to_response(),
            }
        }
        Err(e) => e.to_response(),
    }
}

// ── Query structs ──────────────────────────────────────────────────────────

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

// ── Private helpers ────────────────────────────────────────────────────────

fn get_user(req: &HttpRequest) -> Option<AuthenticatedUser> {
    req.extensions().get::<AuthenticatedUser>().cloned()
}

fn validate_time_range(
    start_time: Option<&str>,
    end_time: Option<&str>,
) -> Result<(Option<i64>, Option<i64>), AppError> {
    let parse_value = |v: &str| -> Result<i64, AppError> {
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

    let start = match start_time {
        Some(s) if !s.is_empty() => Some(parse_value(s)?),
        _ => None,
    };

    let end = match end_time {
        Some(s) if !s.is_empty() => Some(parse_value(s)?),
        _ => None,
    };

    if let (Some(s), Some(e)) = (start, end) {
        if s > e {
            return Err(AppError::bad_request("Invalid time range: start > end"));
        }
    }

    Ok((start, end))
}

async fn ensure_and_resolve(
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
        if profile.permissions.is_none()
            || !profile.permissions.as_ref().map(|p| p.can_view_analytics).unwrap_or(false)
        {
            return Err(AppError::forbidden("Insufficient permissions to view analytics"));
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
        let client_id = user
            .client_id
            .as_ref()
            .ok_or_else(|| AppError::forbidden("Access denied - no client association"))?;

        if client_id.len() != 24 || !client_id.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(AppError::bad_request("Invalid client association"));
        }

        Ok(Some(client_id.clone()))
    }
}
