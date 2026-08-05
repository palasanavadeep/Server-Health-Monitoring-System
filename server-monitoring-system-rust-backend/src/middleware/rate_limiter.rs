use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use actix_web::{
    body::BoxBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use futures::future::{ok, Ready, LocalBoxFuture};
use std::rc::Rc;
use std::sync::Arc;


/// In-memory rate limiter matching express-rate-limit behavior.
/// Tracks request counts per IP within a sliding window.
pub struct RateLimiterState {
    requests: HashMap<String, Vec<Instant>>,
    window_duration: Duration,
    max_requests: u64,
}

impl RateLimiterState {
    pub fn new(window_ms: u64, max_requests: u64) -> Self {
        Self {
            requests: HashMap::new(),
            window_duration: Duration::from_millis(window_ms),
            max_requests,
        }
    }

    /// Check if the IP has exceeded the rate limit. Returns (allowed, remaining, reset_ms).
    pub fn check_rate_limit(&mut self, ip: &str) -> (bool, u64, u64) {
        let now = Instant::now();
        let window_start = now - self.window_duration;

        // Get or create entry for this IP
        let timestamps = self.requests.entry(ip.to_string()).or_default();

        // Remove expired entries
        timestamps.retain(|&t| t > window_start);

        let count = timestamps.len() as u64;

        if count >= self.max_requests {
            // Calculate reset time
            let oldest = timestamps.first().copied().unwrap_or(now);
            let reset_ms = self
                .window_duration
                .checked_sub(now.duration_since(oldest))
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            return (false, 0, reset_ms);
        }

        timestamps.push(now);
        let remaining = self.max_requests - count - 1;
        let reset_ms = self.window_duration.as_millis() as u64;

        (true, remaining, reset_ms)
    }
}

/// Rate limiter middleware factory.
pub struct RateLimiter {
    state: Arc<Mutex<RateLimiterState>>,
    max_requests: u64,
    window_ms: u64,
}

impl RateLimiter {
    pub fn new(window_ms: u64, max_requests: u64) -> Self {
        Self {
            state: Arc::new(Mutex::new(RateLimiterState::new(window_ms, max_requests))),
            max_requests,
            window_ms,
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for RateLimiter
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: actix_web::body::MessageBody + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = RateLimiterMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(RateLimiterMiddleware {
            service: Rc::new(service),
            state: self.state.clone(),
            max_requests: self.max_requests,
            window_ms: self.window_ms,
        })
    }
}

pub struct RateLimiterMiddleware<S> {
    service: Rc<S>,
    state: Arc<Mutex<RateLimiterState>>,
    max_requests: u64,
    window_ms: u64,
}

impl<S, B> Service<ServiceRequest> for RateLimiterMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: actix_web::body::MessageBody + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let state = self.state.clone();
        let max_requests = self.max_requests;
        let window_ms = self.window_ms;

        Box::pin(async move {
            let ip = req
                .connection_info()
                .realip_remote_addr()
                .unwrap_or("unknown")
                .to_string();

            let (allowed, remaining, reset_ms) = {
                let mut limiter = state.lock().unwrap();
                limiter.check_rate_limit(&ip)
            };

            if !allowed {
                // Mirrors Node.js express-rate-limit response format
                let body = serde_json::json!({
                    "success": false,
                    "message": "Too many requests, please try again later",
                    "statusCode": 429
                });

                let mut response = HttpResponse::TooManyRequests().json(body);

                // Standard rate limit headers (standardHeaders: true, legacyHeaders: false)
                let headers = response.headers_mut();
                headers.insert(
                    actix_web::http::header::HeaderName::from_static("ratelimit-limit"),
                    max_requests.to_string().parse().unwrap(),
                );
                headers.insert(
                    actix_web::http::header::HeaderName::from_static("ratelimit-remaining"),
                    "0".parse().unwrap(),
                );
                headers.insert(
                    actix_web::http::header::HeaderName::from_static("ratelimit-reset"),
                    (reset_ms / 1000).to_string().parse().unwrap(),
                );
                headers.insert(
                    actix_web::http::header::RETRY_AFTER,
                    (reset_ms / 1000).to_string().parse().unwrap(),
                );

                return Ok(req.into_response(response).map_into_boxed_body());
            }

            let mut res = service.call(req).await?;

            // Add rate limit headers to successful responses
            let headers = res.headers_mut();
            headers.insert(
                actix_web::http::header::HeaderName::from_static("ratelimit-limit"),
                max_requests.to_string().parse().unwrap(),
            );
            headers.insert(
                actix_web::http::header::HeaderName::from_static("ratelimit-remaining"),
                remaining.to_string().parse().unwrap(),
            );
            headers.insert(
                actix_web::http::header::HeaderName::from_static("ratelimit-reset"),
                (window_ms / 1000).to_string().parse().unwrap(),
            );

            Ok(res.map_into_boxed_body())
        })
    }
}
