use crate::db::DbPool;
use crate::models::auth::jwt::{decode_auth_token, verify_auth_token, verify_device_token, JWTConfig};
use crate::models::auth::scopes::{RequireScope, SessionScope};
use crate::models::auth::session::Session;
use crate::models::http::response::Body;
use crate::models::http::{EMPTY, MESSAGE_INVALID_TOKEN};
use actix_service::forward_ready;
use actix_web::body::EitherBody;
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::header::AUTHORIZATION;
use actix_web::http::Method;
use actix_web::web::Data;
use actix_web::{Error, HttpMessage, HttpResponse};
use futures::future::{ok, LocalBoxFuture, Ready};

pub struct Authentication {
    scopes: &'static [RequireScope],
}

impl Authentication {
    pub fn new(scope: RequireScope) -> Self {
        Self::any_of(match scope {
            RequireScope::Public => &[RequireScope::Public],
            RequireScope::Account => &[RequireScope::Account],
            RequireScope::Device => &[RequireScope::Device],
        })
    }

    /// Allows any listed scope. With Public plus another scope, credentials are
    /// optional, but supplied credentials must match one of the listed session scopes.
    pub fn any_of(scopes: &'static [RequireScope]) -> Self {
        Self { scopes }
    }
}

impl<S, B> Transform<S, ServiceRequest> for Authentication
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Transform = AuthenticationMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthenticationMiddleware {
            service,
            scopes: self.scopes,
        })
    }
}

pub struct AuthenticationMiddleware<S> {
    service: S,
    scopes: &'static [RequireScope],
}

impl<S, B> Service<ServiceRequest> for AuthenticationMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let public = self.scopes.contains(&RequireScope::Public);
        let public_only = self.scopes == [RequireScope::Public];
        let anonymous = public && (public_only || !req.headers().contains_key(AUTHORIZATION));

        if req.method() != Method::OPTIONS && !anonymous {
            let session = match authenticate(&req) {
                Some(session) if self.scopes.iter().any(|scope| scope.allows(&session.scope)) => {
                    session
                }
                Some(_) => {
                    let response = HttpResponse::Forbidden()
                        .json(Body::new("Session scope is not allowed for this route", EMPTY));
                    return Box::pin(async { Ok(req.into_response(response).map_into_right_body()) });
                }
                None => {
                    let response = HttpResponse::Unauthorized()
                        .json(Body::new(MESSAGE_INVALID_TOKEN, EMPTY));
                    return Box::pin(async { Ok(req.into_response(response).map_into_right_body()) });
                }
            };
            req.extensions_mut().insert(session);
        }

        let res = self.service.call(req);
        Box::pin(async move { res.await.map(ServiceResponse::map_into_left_body) })
    }
}

fn authenticate(req: &ServiceRequest) -> Option<Session> {
    let value = req.headers().get(AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    let pool = req.app_data::<Data<DbPool>>()?;
    let config = req.app_data::<Data<JWTConfig>>()?;
    let data = decode_auth_token(token.trim().to_owned(), config).ok()?;
    match &data.claims.scope {
        SessionScope::Account => verify_auth_token(&data, pool).ok(),
        SessionScope::Device(_) => verify_device_token(&data, pool).ok(),
    }
}
