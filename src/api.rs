pub mod poll;
pub mod admin;
pub mod auth;
pub mod federation;
pub mod user;
pub mod relay;
pub mod media;
pub mod videos;

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
use actix_web::{get, web, HttpResponse, Responder};

// ignore routes
pub const IGNORE_ROUTES: [&str; 9] = [
    "/api/auth/register",
    "/api/auth/verify",
    "/api/auth/login",
    "/api/auth/refresh",
    "/api/auth/pass/email",
    "/api/auth/pass/reset",
    "/static",
    "/api/pub/",
    "/metrics",
];

#[get("/metrics")]
async fn metrics() -> impl Responder {
    match crate::sfu::metrics::render() {
        Ok(body) => HttpResponse::Ok()
            .content_type("text/plain; version=0.0.4; charset=utf-8")
            .body(body),
        Err(error) => HttpResponse::InternalServerError().body(error.to_string()),
    }
}

pub fn config_services(cfg: &mut web::ServiceConfig) {
    cfg.service(metrics)
    .service(
        web::scope("/api")
            .service(web::scope("/videos")
                .service(videos::list_owned_videos).service(videos::get_video).service(videos::update_video).service(videos::stop_recording))
            .service(web::scope("/streams")
                .service(videos::list_videos).service(videos::create_video).service(videos::start_recording))
            .service(poll::poll)
            .service(web::scope("/messages")
                .service(crate::messages::api::list).service(crate::messages::api::counts)
                .service(crate::messages::api::read_all).service(crate::messages::api::read))
            .service(web::scope("/notifications")
                .service(crate::notifications::api::list).service(crate::notifications::api::counts)
                .service(crate::notifications::api::read_all).service(crate::notifications::api::read))
            .service(
                web::scope("/admin")
                    .service(admin::monitoring)
                    .service(admin::list_users)
                    .service(admin::set_user_active)
                    .service(admin::set_user_role),
            )
            .service(
                web::scope("/pub")
                    .service(web::scope("/videos").service(videos::get_public_video))
                    .service(web::scope("/stream")
                        .service(media::available))
                    .service(
                        web::scope("/user")
                            .service(search_active_users_by_name)
                            .service(get_active_user),
                    )
                    .service(
                        web::scope("/channel")
                            .service(channel::get_channel)
                            .service(stream_preview::get_channel_stream_preview_list)
                            .service(user::recorded_video_preview::get_channel_recorded_video_preview_list),
                    )
                    .service(
                        web::scope("/stream-preview")
                            .service(stream_preview::get_stream_preview_list)
                            .service(stream_preview::get_stream_preview),
                    )
                    .service(web::scope("/recorded-video-preview")
                        .service(user::recorded_video_preview::get_recorded_video_preview_list)
                        .service(user::recorded_video_preview::get_recorded_video_preview))
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
                    .service(lobby::host_presence)
                    .service(lobby::start_streaming)
                    .service(lobby::stop_streaming),
            )
            .service(
                web::scope("/stream")
                    .service(media::status).service(media::start).service(media::stop).service(media::publication)
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
