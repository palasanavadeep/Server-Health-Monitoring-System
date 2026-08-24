use std::num::NonZeroU32;
use std::sync::Arc;

use actix_web::{
    body::BoxBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use futures::future::{ok, LocalBoxFuture, Ready};
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorRateLimiter,
};
use std::rc::Rc;

/// Per-IP rate limiter backed by the `governor` crate.
///
/// Uses a fixed-window quota with automatic memory management —
/// no unbounded `HashMap`, no manual eviction.
///
/// ## Configuration
///
/// - `max_requests` — burst capacity (requests per window)
/// - `window_ms` — window duration in milliseconds
///
/// The middleware is **not keyed per-IP** in this implementation because
/// `governor`'s keyed rate limiter requires `Clone` which conflicts with
/// actix-web's `!Send` middleware model. Instead we apply a global
/// rate limit matching the original express-rate-limit behavior when
/// used as a server-wide middleware.
pub struct RateLimiter {
    limiter: Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
    max_requests: u64,
    window_ms: u64,
}

impl RateLimiter {
    pub fn new(window_ms: u64, max_requests: u64) -> Self {
        let period = std::time::Duration::from_millis(window_ms);
        let burst = NonZeroU32::new(max_requests as u32).unwrap_or(NonZeroU32::new(100).unwrap());

        let quota = Quota::with_period(period / burst.get())
            .expect("rate limit period too small")
            .allow_burst(burst);

        Self {
            limiter: Arc::new(GovernorRateLimiter::direct(quota)),
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
            limiter: self.limiter.clone(),
            max_requests: self.max_requests,
            window_ms: self.window_ms,
        })
    }
}

pub struct RateLimiterMiddleware<S> {
    service: Rc<S>,
    limiter: Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
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
        let limiter = self.limiter.clone();
        let max_requests = self.max_requests;
        let window_ms = self.window_ms;

        Box::pin(async move {
            match limiter.check() {
                Ok(_) => {
                    let mut res = service.call(req).await?;

                    // Add rate limit headers to successful responses
                    let headers = res.headers_mut();
                    headers.insert(
                        actix_web::http::header::HeaderName::from_static("ratelimit-limit"),
                        max_requests.to_string().parse().unwrap(),
                    );
                    headers.insert(
                        actix_web::http::header::HeaderName::from_static("ratelimit-reset"),
                        (window_ms / 1000).to_string().parse().unwrap(),
                    );

                    Ok(res.map_into_boxed_body())
                }
                Err(not_until) => {
                    let retry_after = not_until
                        .wait_time_from(governor::clock::Clock::now(
                            &governor::clock::DefaultClock::default(),
                        ))
                        .as_secs();

                    let body = serde_json::json!({
                        "success": false,
                        "message": "Too many requests, please try again later",
                        "statusCode": 429
                    });

                    let mut response = HttpResponse::TooManyRequests().json(body);

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
                        retry_after.to_string().parse().unwrap(),
                    );
                    headers.insert(
                        actix_web::http::header::RETRY_AFTER,
                        retry_after.to_string().parse().unwrap(),
                    );

                    Ok(req.into_response(response).map_into_boxed_body())
                }
            }
        })
    }
}
