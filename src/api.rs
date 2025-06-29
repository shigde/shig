pub mod auth;
pub mod federation;
pub mod user;

use crate::api::auth::login::login;
use crate::api::auth::pass::{reset_password, send_forgotten_pass_email, update_password};
use crate::api::auth::refresh::refresh;
use crate::api::auth::signup::signup;
use crate::api::auth::user::{delete_current_user, get_current_user};
use crate::api::auth::verify::verify;
use crate::api::federation::settings::get_settings;
use crate::api::user::channel::{get_channel, update_channel};
use crate::api::user::get_active_user;
use crate::api::user::stream::{create_stream, delete_stream, get_stream, update_stream};
use crate::api::user::stream_preview::{
    get_channel_stream_preview_list, get_stream_preview, get_stream_preview_list,
};
use actix_files as fs;
use actix_web::web;

// ignore routes
pub const IGNORE_ROUTES: [&str; 8] = [
    "/api/auth/register",
    "/api/auth/verify",
    "/api/auth/login",
    "/api/auth/refresh",
    "/api/auth/pass/email",
    "/api/auth/pass/reset",
    "/static",
    "/api/pub/",
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
                web::scope("/pub")
                    .service(web::scope("/user").service(get_active_user))
                    .service(
                        web::scope("/channel")
                            .service(get_channel)
                            .service(get_channel_stream_preview_list),
                    )
                    .service(
                        web::scope("/stream-preview")
                            .service(get_stream_preview_list)
                            .service(get_stream_preview),
                    )
                    .service(web::scope("/federation").service(get_settings)),
            )
            .service(web::scope("/channel").service(update_channel))
            .service(
                web::scope("/stream")
                    .service(get_stream)
                    .service(create_stream)
                    .service(update_stream)
                    .service(delete_stream),
            )
            .service(
                web::scope("/auth")
                    .service(login)
                    .service(signup)
                    .service(get_current_user)
                    .service(delete_current_user)
                    .service(verify)
                    .service(refresh)
                    .service(send_forgotten_pass_email)
                    .service(reset_password)
                    .service(update_password),
            ),
    );
}
