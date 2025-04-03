pub mod auth;
pub mod user;
use crate::api::auth::login::login;
use crate::api::auth::pass::{reset_password, send_forgotten_pass_email, update_password};
use crate::api::auth::refresh::refresh;
use crate::api::auth::signup::signup;
use crate::api::auth::user::{delete_current_user, get_current_user};
use crate::api::auth::verify::verify;
use crate::api::user::channel::{get_channel, update_channel};
use actix_files as fs;
use actix_web::web;

// ignore routes
pub const IGNORE_ROUTES: [&str; 7] = [
    "/api/auth/register",
    "/api/auth/verify",
    "/api/auth/login",
    "/api/auth/refresh",
    "/api/auth/pass/email",
    "/api/auth/pass/reset",
    "/static",
];

pub fn config_services(cfg: &mut web::ServiceConfig) {
    cfg.service(
        fs::Files::new("/static", "./htdocs")
            // .show_files_listing()
            .use_last_modified(true),
    )
    .service(
        web::scope("/api")
            .service(
                web::resource("/channel")
                    .route(web::get().to(get_channel))
                    .route(web::put().to(update_channel)),
            )
            .service(
                web::scope("/auth")
                    .service(web::resource("/login").route(web::post().to(login)))
                    .service(web::resource("/register").route(web::post().to(signup)))
                    .service(
                        web::resource("/user")
                            .route(web::get().to(get_current_user))
                            .route(web::delete().to(delete_current_user)),
                    )
                    .service(web::resource("/verify/{token}").route(web::get().to(verify)))
                    .service(web::resource("/refresh").route(web::post().to(refresh)))
                    .service(
                        web::resource("/pass/email")
                            .route(web::post().to(send_forgotten_pass_email)),
                    )
                    .service(web::resource("/pass/reset").route(web::put().to(reset_password)))
                    .service(web::resource("/pass/update").route(web::put().to(update_password))),
            ),
    );
}
