use actix_web::{
    body::BoxBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures::future::{ok, LocalBoxFuture, Ready};
use std::rc::Rc;
use std::time::Instant;
use uuid::Uuid;

/// Request correlation ID — inserted into request extensions and response headers.
///
/// Downstream handlers can extract this from `req.extensions().get::<RequestId>()`
/// to correlate logs across the request lifecycle.
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

/// Request logger middleware — logs method, path, status, duration, and
/// a unique correlation ID per request.
pub struct RequestLogger;

impl<S, B> Transform<S, ServiceRequest> for RequestLogger
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: actix_web::body::MessageBody + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = RequestLoggerMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(RequestLoggerMiddleware {
            service: Rc::new(service),
        })
    }
}

pub struct RequestLoggerMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for RequestLoggerMiddleware<S>
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
        let method = req.method().to_string();
        let path = req.path().to_string();
        let ip = req
            .connection_info()
            .realip_remote_addr()
            .unwrap_or("unknown")
            .to_string();

        // Generate or extract correlation ID
        let request_id = req
            .headers()
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        // Inject into request extensions for downstream handlers
        req.extensions_mut().insert(RequestId(request_id.clone()));

        let start = Instant::now();

        Box::pin(async move {
            let mut res = service.call(req).await?;
            let elapsed = start.elapsed().as_millis();
            let status = res.status().as_u16();

            // Attach correlation ID to response
            res.headers_mut().insert(
                actix_web::http::header::HeaderName::from_static("x-request-id"),
                request_id.parse().unwrap(),
            );

            tracing::info!(
                request_id = %request_id,
                method = %method,
                path = %path,
                status = status,
                duration_ms = elapsed,
                ip = %ip,
                "HTTP request"
            );

            Ok(res.map_into_boxed_body())
        })
    }
}
