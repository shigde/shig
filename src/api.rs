pub mod auth;
pub mod users;

use crate::api::auth::login::login;
use actix_web::web;

// ignore routes
pub const IGNORE_ROUTES: [&str; 2] = ["/api/auth/signup", "/api/auth/login"];

pub fn config_services(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/api").service(
        web::scope("/auth").service(web::resource("/login").route(web::post().to(login))),
    ));
}
