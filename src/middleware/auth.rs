use crate::api::IGNORE_ROUTES;
use crate::db::DbPool;
use crate::models::auth::jwt::{decode_auth_token, verify_auth_token, JWTConfig};
use crate::models::http::response::Body;
use crate::models::http::{EMPTY, MESSAGE_INVALID_TOKEN};
use actix_service::forward_ready;
use actix_web::body::EitherBody;
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::header::AUTHORIZATION;
use actix_web::http::{
    header::{HeaderName, HeaderValue},
    Method,
};
use actix_web::web::Data;
use actix_web::HttpResponse;
use actix_web::{Error, HttpMessage};
use futures::future::{ok, LocalBoxFuture, Ready};
use log::{error, info};

pub struct Authentication;

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
        ok(AuthenticationMiddleware { service })
    }
}

pub struct AuthenticationMiddleware<S> {
    service: S,
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
        let mut authenticate_pass: bool = false;

        // Bypass some account routes
        let mut headers = req.headers().clone();
        headers.append(
            HeaderName::from_static("content-length"),
            HeaderValue::from_static("true"),
        );
        if Method::OPTIONS == *req.method() {
            authenticate_pass = true;
        } else {
            for ignore_route in IGNORE_ROUTES.iter() {
                if req.path().starts_with(ignore_route) {
                    authenticate_pass = true;
                    break;
                }
            }
        }

        if !authenticate_pass {
            if let Some(pool) = req.app_data::<Data<DbPool>>() {
                info!("Connecting to database...");
                if let Some(jwt_config) = req.app_data::<Data<JWTConfig>>() {
                    if let Some(authen_header) = req.headers().get(AUTHORIZATION) {
                        info!("Parsing authorization header...");
                        if let Ok(authen_str) = authen_header.to_str() {
                            if authen_str.starts_with("bearer") || authen_str.starts_with("Bearer")
                            {
                                info!("Parsing token...");
                                let token = authen_str[6..authen_str.len()].trim();
                                if let Ok(token_data) =
                                    decode_auth_token(token.to_string(), jwt_config)
                                {
                                    info!("Decoding token...");
                                    let session = verify_auth_token(&token_data, pool);
                                    if session.is_ok() {
                                        info!("Valid token");
                                        req.extensions_mut().insert(session.unwrap());
                                        authenticate_pass = true;
                                    } else {
                                        error!("Invalid token");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if !authenticate_pass {
            let (request, _pl) = req.into_parts();

            let mut response_code = HttpResponse::Unauthorized();
            // if request.path().starts_with("/api/auth/user") {
            //     response_code = HttpResponse::Forbidden();
            // }

            let response = response_code
                .json(Body::new(MESSAGE_INVALID_TOKEN, EMPTY))
                .map_into_right_body();

            return Box::pin(async { Ok(ServiceResponse::new(request, response)) });
        }

        let res = self.service.call(req);

        Box::pin(async move { res.await.map(ServiceResponse::map_into_left_body) })
    }
}
