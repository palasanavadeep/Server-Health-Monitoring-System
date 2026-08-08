use actix_web::{
    body::BoxBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse,
};
use futures::future::{ok, Ready, LocalBoxFuture};
use std::rc::Rc;

use crate::middleware::authenticate::AuthenticatedUser;
use crate::util::response::ResponseFormatter;

/// Authorize middleware factory - checks user role against allowed roles.
/// Mirrors Node.js authorize.js middleware.
pub struct Authorize {
    pub allowed_roles: Vec<String>,
}

impl Authorize {
    pub fn new(roles: Vec<&str>) -> Self {
        Self {
            allowed_roles: roles.into_iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for Authorize
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: actix_web::body::MessageBody + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = AuthorizeMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthorizeMiddleware {
            service: Rc::new(service),
            allowed_roles: self.allowed_roles.clone(),
        })
    }
}

pub struct AuthorizeMiddleware<S> {
    service: Rc<S>,
    allowed_roles: Vec<String>,
}

impl<S, B> Service<ServiceRequest> for AuthorizeMiddleware<S>
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
        let allowed_roles = self.allowed_roles.clone();

        Box::pin(async move {
            // Get authenticated user from request extensions
            let user = req.extensions().get::<AuthenticatedUser>().cloned();

            match user {
                Some(user) => {
                    if allowed_roles.contains(&user.role) {
                        let res = service.call(req).await?;
                        Ok(res.map_into_boxed_body())
                    } else {
                        let body = ResponseFormatter::error(
                            "Insufficient permissions",
                            403,
                            None,
                        );
                        let response = HttpResponse::Forbidden().json(body);
                        Ok(req.into_response(response).map_into_boxed_body())
                    }
                }
                None => {
                    let body = ResponseFormatter::error("Forbidden", 403, None);
                    let response = HttpResponse::Forbidden().json(body);
                    Ok(req.into_response(response).map_into_boxed_body())
                }
            }
        })
    }
}
