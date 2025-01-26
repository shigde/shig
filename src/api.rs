pub mod auth;
pub mod users;

use crate::api::auth::login::login;
use crate::api::auth::signup::signup;
use crate::api::auth::verify::verify;
use actix_web::web;

// ignore routes
pub const IGNORE_ROUTES: [&str; 3] = ["/api/auth/register", "/api/auth/verify", "/api/auth/login"];

pub fn config_services(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api").service(
            web::scope("/auth")
                .service(web::resource("/login").route(web::post().to(login)))
                .service(web::resource("/register").route(web::post().to(signup)))
                .service(web::resource("/verify/{token}").route(web::get().to(verify))),
        ),
    );
}
