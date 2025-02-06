pub mod auth;
pub mod user;
use crate::api::auth::login::login;
use crate::api::auth::refresh::refresh;
use crate::api::auth::signup::signup;
use crate::api::auth::user::get_current_user;
use crate::api::auth::verify::verify;
use actix_web::web;

// ignore routes
pub const IGNORE_ROUTES: [&str; 4] = [
    "/api/auth/register",
    "/api/auth/verify",
    "/api/auth/login",
    "/api/auth/refresh",
];

pub fn config_services(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api").service(
            web::scope("/auth")
                .service(web::resource("/login").route(web::post().to(login)))
                .service(web::resource("/register").route(web::post().to(signup)))
                .service(web::resource("/user").route(web::get().to(get_current_user)))
                .service(web::resource("/verify/{token}").route(web::get().to(verify)))
                .service(web::resource("/refresh").route(web::post().to(refresh))),
        ),
    );
}
