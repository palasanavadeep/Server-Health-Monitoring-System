use actix_web::{
    body::BoxBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse,
};
use futures::future::{ok, LocalBoxFuture, Ready};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use std::rc::Rc;

use crate::config::settings::AppConfig;
use crate::domain::user::JwtClaims;
use crate::util::response::ResponseFormatter;

/// Data extracted from JWT and attached to request extensions.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id:            String,
    pub email:              String,
    pub username:           String,
    pub role:               String,
    pub client_id:          Option<String>,
    /// Derived from role claim — true when role == "super_admin".
    pub is_super_admin:     bool,
    /// Embedded at login time; eliminates DB round-trip for analytics permission checks.
    pub can_view_analytics: bool,
}

/// Authenticate middleware factory.
/// Mirrors Node.js authenticate.js — reads JWT from `authToken` cookie.
pub struct Authenticate {
    pub jwt_secret: String,
}

impl Authenticate {
    pub fn new(config: &AppConfig) -> Self {
        Self {
            jwt_secret: config.jwt.secret.clone(),
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for Authenticate
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: actix_web::body::MessageBody + 'static,
{
    // Always resolve to BoxBody so both success and error paths are compatible
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = AuthenticateMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthenticateMiddleware {
            service: Rc::new(service),
            jwt_secret: self.jwt_secret.clone(),
        })
    }
}

pub struct AuthenticateMiddleware<S> {
    service: Rc<S>,
    jwt_secret: String,
}

impl<S, B> Service<ServiceRequest> for AuthenticateMiddleware<S>
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
        let jwt_secret = self.jwt_secret.clone();

        Box::pin(async move {
            // Extract token from authToken cookie (mirrors Node.js: req.cookies.authToken)
            let token = req.cookie("authToken").map(|c| c.value().to_string());

            let token = match token {
                Some(t) if !t.is_empty() => t,
                _ => {
                    let body =
                        ResponseFormatter::error("Authentication token is required", 401, None);
                    let response = HttpResponse::Unauthorized().json(body);
                    return Ok(req.into_response(response).map_into_boxed_body());
                }
            };

            // Decode and verify JWT
            let decoding_key = DecodingKey::from_secret(jwt_secret.as_bytes());
            let mut validation = Validation::new(Algorithm::HS256);
            validation.validate_exp = true;

            match decode::<JwtClaims>(&token, &decoding_key, &validation) {
                Ok(token_data) => {
                    let claims = token_data.claims;

                    let user = AuthenticatedUser {
                        user_id:            claims.user_id,
                        email:              claims.email,
                        username:           claims.username,
                        role:               claims.role,
                        client_id:          claims.client_id,
                        is_super_admin:     claims.is_super_admin,
                        can_view_analytics: claims.can_view_analytics,
                    };

                    // Attach user to request extensions
                    req.extensions_mut().insert(user);

                    let res = service.call(req).await?;
                    Ok(res.map_into_boxed_body())
                }
                Err(err) => {
                    use jsonwebtoken::errors::ErrorKind;
                    let message = match err.kind() {
                        ErrorKind::ExpiredSignature => "Token expired",
                        _ => "Invalid token",
                    };

                    let body = ResponseFormatter::error(message, 401, None);
                    let response = HttpResponse::Unauthorized().json(body);
                    Ok(req.into_response(response).map_into_boxed_body())
                }
            }
        })
    }
}
