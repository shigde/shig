pub mod poll;
pub mod admin;
pub mod auth;
pub mod devices;
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
use crate::middleware::auth::Authentication;
use crate::models::auth::scopes::RequireScope;
use actix_web::{web, HttpResponse, Responder};

async fn metrics() -> impl Responder {
    match crate::sfu::metrics::render() {
        Ok(body) => HttpResponse::Ok()
            .content_type("text/plain; version=0.0.4; charset=utf-8")
            .body(body),
        Err(error) => HttpResponse::InternalServerError().body(error.to_string()),
    }
}

pub fn config_services(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/metrics")
            .wrap(Authentication::new(RequireScope::Public))
            .route(web::get().to(metrics)),
    )
    .service(
        web::scope("/api")
            .service(web::scope("/devices")
                .wrap(Authentication::new(RequireScope::Account))
                .service(devices::list)
                .service(devices::create)
                .service(devices::update)
                .service(devices::remove)
                .service(devices::invite))
            .service(web::scope("/videos")
                .wrap(Authentication::new(RequireScope::Account))
                .service(videos::list_owned_videos).service(videos::get_video).service(videos::update_video).service(videos::delete_video).service(videos::stop_recording))
            .service(web::scope("/streams")
                .wrap(Authentication::new(RequireScope::Account))
                .service(videos::list_videos).service(videos::create_video).service(videos::recording_ready).service(videos::start_recording))
            .service(web::scope("/messages")
                .wrap(Authentication::new(RequireScope::Account))
                .service(crate::messages::api::list).service(crate::messages::api::counts)
                .service(crate::messages::api::read_all).service(crate::messages::api::read))
            .service(web::scope("/notifications")
                .wrap(Authentication::new(RequireScope::Account))
                .service(crate::notifications::api::list).service(crate::notifications::api::counts)
                .service(crate::notifications::api::read_all).service(crate::notifications::api::read))
            .service(
                web::scope("/admin")
                    .wrap(Authentication::new(RequireScope::Account))
                    .service(admin::monitoring)
                    .service(admin::list_users)
                    .service(admin::set_user_active)
                    .service(admin::set_user_role),
            )
            .service(
                web::scope("/pub")
                    .wrap(Authentication::any_of(&[RequireScope::Public, RequireScope::Account]))
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
                    .wrap(Authentication::new(RequireScope::Account))
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
                    .wrap(Authentication::new(RequireScope::Account))
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
                    .service(web::resource("/device/redeem")
                        .wrap(Authentication::new(RequireScope::Public))
                        .route(web::post().to(devices::redeem)))
                    .service(web::resource("/device/disconnect")
                        .wrap(Authentication::new(RequireScope::Device))
                        .route(web::post().to(devices::disconnect)))
                    .service(web::resource("/device/whip")
                        .wrap(Authentication::new(RequireScope::Device))
                        .route(web::post().to(devices::whip)))
                    .service(web::resource("/user")
                        .wrap(Authentication::new(RequireScope::Account))
                        .route(web::get().to(get_current_user))
                        .route(web::delete().to(delete_current_user)))
                    .service(web::resource("/pass/update")
                        .wrap(Authentication::new(RequireScope::Account))
                        .route(web::put().to(update_password)))
                    .service(web::scope("")
                        .wrap(Authentication::new(RequireScope::Public))
                        .service(login)
                        .service(signup)
                        .service(verify)
                        .service(refresh)
                        .service(send_forgotten_pass_email)
                        .service(reset_password)),
            )
            .service(web::scope("")
                .wrap(Authentication::new(RequireScope::Account))
                .service(poll::poll)),

    );
}

#[cfg(test)]
mod routing_tests {
    #[actix_web::test]
    async fn protected_routes_require_a_session() {
        use actix_web::{http::{Method, StatusCode}, test, App};
        let app = test::init_service(App::new().configure(super::config_services)).await;
        for (method, path) in [
            (Method::GET, "/api/videos/00000000-0000-0000-0000-000000000000"),
            (Method::PATCH, "/api/videos/00000000-0000-0000-0000-000000000000"),
            (Method::DELETE, "/api/videos/00000000-0000-0000-0000-000000000000"),
            (Method::GET, "/api/devices"),
            (Method::POST, "/api/auth/device/whip"),
            (Method::POST, "/api/auth/device/disconnect"),
            (Method::GET, "/api/auth/user"),
            (Method::POST, "/api/poll"),
        ] {
            let response = test::call_service(&app, test::TestRequest::default()
                .method(method.clone()).uri(path)
                .set_json(serde_json::json!({})).to_request()).await;
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{method} {path}");
        }
        let redeem = test::call_service(&app, test::TestRequest::post()
            .uri("/api/auth/device/redeem").set_json(serde_json::json!({"token": "test"}))
            .to_request()).await;
        // The request reaches the handler, which has no database in this test.
        assert_eq!(redeem.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[actix_web::test]
    async fn public_routes_support_anonymous_and_optional_account_access() {
        use actix_web::{http::StatusCode, test, App};
        let app = test::init_service(App::new().configure(super::config_services)).await;
        let path = "/api/pub/federation/settings";
        let anonymous = test::call_service(&app, test::TestRequest::get().uri(path).to_request()).await;
        // Missing settings data produces a handler error, so the anonymous request passed auth.
        assert_eq!(anonymous.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let invalid_token = test::call_service(
            &app,
            test::TestRequest::get()
                .uri(path)
                .insert_header(("Authorization", "Bearer invalid"))
                .to_request(),
        ).await;
        assert_eq!(invalid_token.status(), StatusCode::UNAUTHORIZED);
    }
}
