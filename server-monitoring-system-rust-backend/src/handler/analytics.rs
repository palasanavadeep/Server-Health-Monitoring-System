use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};

use crate::app_state::AppState;
use crate::error::app_error::AppError;
use crate::middleware::authenticate::AuthenticatedUser;
use crate::util::response::ResponseFormatter;

// ── Query parameter structs ───────────────────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct TimeRangeQuery {
    #[serde(rename = "startTime")]
    pub start_time: Option<String>,
    #[serde(rename = "endTime")]
    pub end_time: Option<String>,
    #[serde(rename = "clientId")]
    pub client_id: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct EndpointListQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    #[serde(rename = "clientId")]
    pub client_id: Option<String>,
}

/// Query parameters for `GET /api/analytics/percentiles`.
///
/// `service_name`, `endpoint`, and `method` identify a single endpoint.
/// `start_time` / `end_time` default to the last 24 hours when absent.
#[derive(serde::Deserialize)]
pub struct EndpointMetricsQuery {
    #[serde(rename = "startTime")]
    pub start_time: Option<String>,
    #[serde(rename = "endTime")]
    pub end_time: Option<String>,
    #[serde(rename = "clientId")]
    pub client_id: Option<String>,
    #[serde(rename = "serviceName")]
    pub service_name: String,
    pub endpoint: String,
    pub method: String,
}

/// Query parameters for `GET /api/analytics/services`.
///
/// - Without `serviceName` → returns health overview for all services.
/// - With    `serviceName` → returns full metrics for that single service.
#[derive(serde::Deserialize)]
pub struct ServiceQuery {
    #[serde(rename = "startTime")]
    pub start_time: Option<String>,
    #[serde(rename = "endTime")]
    pub end_time: Option<String>,
    #[serde(rename = "clientId")]
    pub client_id: Option<String>,
    #[serde(rename = "serviceName")]
    pub service_name: Option<String>,
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn authenticated_user(req: &HttpRequest) -> Option<AuthenticatedUser> {
    req.extensions().get::<AuthenticatedUser>().cloned()
}

fn parse_time_range(
    start: Option<&str>,
    end: Option<&str>,
) -> Result<(Option<i64>, Option<i64>), AppError> {
    let parse_one = |v: &str| -> Result<i64, AppError> {
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
        Some(v) if !v.is_empty() => Some(parse_one(v)?),
        _ => None,
    };
    let e = match end {
        Some(v) if !v.is_empty() => Some(parse_one(v)?),
        _ => None,
    };

    if let (Some(s), Some(e)) = (s, e) {
        if s > e {
            return Err(AppError::bad_request("start_time must be before end_time"));
        }
    }
    Ok((s, e))
}

/// Resolve the effective `client_id` for the authenticated user — **zero DB calls**.
///
/// `is_super_admin` and `can_view_analytics` are read from `AuthenticatedUser`,
/// which is populated from the JWT at middleware time. No MongoDB round-trip needed.
///
/// - Super-admins may pass any valid `client_id` via query param, or omit it.
/// - Regular users must have `can_view_analytics = true` from their JWT claims.
fn resolve_client_id(
    user: &AuthenticatedUser,
    query_client_id: Option<&str>,
) -> Result<Option<String>, AppError> {
    if !user.is_super_admin && !user.can_view_analytics {
        return Err(AppError::forbidden("Insufficient permissions to view analytics"));
    }

    if user.is_super_admin {
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
            .ok_or_else(|| AppError::forbidden("Access denied: no client association"))?;
        Ok(Some(cid.clone()))
    }
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// `GET /api/analytics/stats`
pub async fn get_stats(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<TimeRangeQuery>,
) -> HttpResponse {
    let user = match authenticated_user(&req) {
        Some(u) => u,
        None => return HttpResponse::Unauthorized()
            .json(ResponseFormatter::unauthorized("Authentication required")),
    };
    let client_id = match resolve_client_id(&user, query.client_id.as_deref()) {
        Ok(c)  => c,
        Err(e) => return e.to_response(),
    };
    let (start, end) = match parse_time_range(query.start_time.as_deref(), query.end_time.as_deref()) {
        Ok(r)  => r,
        Err(e) => return e.to_response(),
    };

    match state.analytics_service.get_overall_stats(client_id.as_deref(), start, end).await {
        Ok(stats) => HttpResponse::Ok()
            .json(ResponseFormatter::ok(stats, "Statistics retrieved successfully")),
        Err(e) => e.to_response(),
    }
}

/// `GET /api/analytics/dashboard`
pub async fn get_dashboard(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<TimeRangeQuery>,
) -> HttpResponse {
    let user = match authenticated_user(&req) {
        Some(u) => u,
        None => return HttpResponse::Unauthorized()
            .json(ResponseFormatter::unauthorized("Authentication required")),
    };
    let client_id = match resolve_client_id(&user, query.client_id.as_deref()) {
        Ok(c)  => c,
        Err(e) => return e.to_response(),
    };
    let (start, end) = match parse_time_range(query.start_time.as_deref(), query.end_time.as_deref()) {
        Ok(r)  => r,
        Err(e) => return e.to_response(),
    };

    let dashboard = state
        .analytics_service
        .get_dashboard(client_id.as_deref(), start, end)
        .await;

    HttpResponse::Ok().json(ResponseFormatter::ok(dashboard, "Dashboard retrieved successfully"))
}

/// `GET /api/analytics/apis`
pub async fn get_endpoint_list(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<EndpointListQuery>,
) -> HttpResponse {
    let user = match authenticated_user(&req) {
        Some(u) => u,
        None => return HttpResponse::Unauthorized()
            .json(ResponseFormatter::unauthorized("Authentication required")),
    };
    let client_id = match resolve_client_id(&user, query.client_id.as_deref()) {
        Ok(c)  => c,
        Err(e) => return e.to_response(),
    };
    let client_id = match client_id {
        Some(id) => id,
        None => return HttpResponse::BadRequest()
            .json(ResponseFormatter::bad_request("clientId is required", None)),
    };

    let page  = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(10);

    let tenant_config = match state.tenant_config_service.get_config(&client_id).await {
        Ok(c)  => c,
        Err(e) => return e.to_response(),
    };

    match state.analytics_service.get_endpoint_metrics_page(&client_id, &tenant_config, page, limit).await {
        Ok((items, total, current_page, page_limit)) =>
            HttpResponse::Ok().json(ResponseFormatter::paged(items, current_page, page_limit, total)),
        Err(e) => e.to_response(),
    }
}

/// `GET /api/analytics/percentiles`
///
/// Returns p50/p75/p90/p95/p99 latency percentiles, Apdex score, HTTP status
/// class distribution, and throughput for a single endpoint over a time range.
///
/// **Required params:** `serviceName`, `endpoint`, `method`
/// **Optional params:** `startTime`, `endTime`, `clientId`
pub async fn get_endpoint_metrics(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<EndpointMetricsQuery>,
) -> HttpResponse {
    let user = match authenticated_user(&req) {
        Some(u) => u,
        None => return HttpResponse::Unauthorized()
            .json(ResponseFormatter::unauthorized("Authentication required")),
    };
    let client_id = match resolve_client_id(&user, query.client_id.as_deref()) {
        Ok(c)  => c,
        Err(e) => return e.to_response(),
    };
    let client_id = match client_id {
        Some(id) => id,
        None => return HttpResponse::BadRequest()
            .json(ResponseFormatter::bad_request("clientId is required", None)),
    };
    let (start, end) = match parse_time_range(query.start_time.as_deref(), query.end_time.as_deref()) {
        Ok(r)  => r,
        Err(e) => return e.to_response(),
    };

    let tenant_config = match state.tenant_config_service.get_config(&client_id).await {
        Ok(c)  => c,
        Err(e) => return e.to_response(),
    };

    match state
        .analytics_service
        .get_endpoint_metrics(
            &client_id, &query.service_name, &query.endpoint, &query.method,
            &tenant_config, start, end,
        )
        .await
    {
        Ok(metrics) => HttpResponse::Ok()
            .json(ResponseFormatter::ok(metrics, "Endpoint metrics retrieved successfully")),
        Err(e) => e.to_response(),
    }
}

/// `GET /api/analytics/services`
///
/// Dual-purpose endpoint controlled by the optional `serviceName` query param:
/// - Without `?serviceName=…` → health overview for **all** services (fleet view).
/// - With    `?serviceName=…` → full metrics for that **single** service (detail view).
///
/// Both paths require a `clientId` for non-super-admin callers.
pub async fn get_services(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<ServiceQuery>,
) -> HttpResponse {
    let user = match authenticated_user(&req) {
        Some(u) => u,
        None => return HttpResponse::Unauthorized()
            .json(ResponseFormatter::unauthorized("Authentication required")),
    };
    let client_id = match resolve_client_id(&user, query.client_id.as_deref()) {
        Ok(c)  => c,
        Err(e) => return e.to_response(),
    };
    let client_id = match client_id {
        Some(id) => id,
        None => return HttpResponse::BadRequest()
            .json(ResponseFormatter::bad_request("clientId is required", None)),
    };
    let (start, end) = match parse_time_range(query.start_time.as_deref(), query.end_time.as_deref()) {
        Ok(r)  => r,
        Err(e) => return e.to_response(),
    };

    let tenant_config = match state.tenant_config_service.get_config(&client_id).await {
        Ok(c)  => c,
        Err(e) => return e.to_response(),
    };

    if let Some(ref service_name) = query.service_name {
        match state
            .analytics_service
            .get_service_metrics(&client_id, service_name, &tenant_config, start, end)
            .await
        {
            Ok(metrics) => HttpResponse::Ok()
                .json(ResponseFormatter::ok(metrics, "Service metrics retrieved successfully")),
            Err(e) => e.to_response(),
        }
    } else {
        match state
            .analytics_service
            .get_services_health(&client_id, &tenant_config, start, end)
            .await
        {
            Ok(services) => HttpResponse::Ok()
                .json(ResponseFormatter::ok(services, "Service health overview retrieved successfully")),
            Err(e) => e.to_response(),
        }
    }
}
