pub mod auth;
pub mod federation;
pub mod user;
pub mod relay;

use crate::api::auth::login::login;
use crate::api::auth::pass::{reset_password, send_forgotten_pass_email, update_password};
use crate::api::auth::refresh::refresh;
use crate::api::auth::signup::signup;
use crate::api::auth::user::{delete_current_user, get_current_user};
use crate::api::auth::verify::verify;
use crate::api::federation::settings::get_settings;
use crate::api::user::stream::{create_stream, delete_stream, get_stream, update_stream};
use crate::api::user::stream_friend::{
    create_stream_friend, delete_stream_friend, get_all_stream_friends, get_stream_friend,
};
use crate::api::user::{
    channel, get_active_user, lobby, search_active_users_by_name, stream_preview, whep, whip,
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
                    .service(
                        web::scope("/user")
                            .service(search_active_users_by_name)
                            .service(get_active_user),
                    )
                    .service(
                        web::scope("/channel")
                            .service(channel::get_channel)
                            .service(stream_preview::get_channel_stream_preview_list),
                    )
                    .service(
                        web::scope("/stream-preview")
                            .service(stream_preview::get_stream_preview_list)
                            .service(stream_preview::get_stream_preview),
                    )
                    .service(
                        web::scope("/streaming")
                            .service(relay::announcement::get_root)
                            .service(relay::announcement::get_prefix),
                    )
                    .service(web::scope("/federation").service(get_settings)),
            )
            .service(
                web::scope("/channel")
                    .service(channel::update_channel)
                    .service(whip::create_answer)
                    .service(whep::create_offer)
                    .service(whep::set_answer)
                    .service(lobby::participant_leave)
                    .service(lobby::participants_list)
                    .service(lobby::is_online)
                    .service(lobby::start_streaming)
                    .service(lobby::stop_streaming),
            )
            .service(
                web::scope("/stream")
                    .service(get_stream)
                    .service(create_stream)
                    .service(update_stream)
                    .service(delete_stream)
                    .service(get_stream_friend)
                    .service(get_all_stream_friends)
                    .service(create_stream_friend)
                    .service(delete_stream_friend),
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
