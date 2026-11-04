use crate::{
    db::{recorded_videos, streams::read::find_stream_by_uuid, DbPool},
    models::{
        auth::session::Session, error::ApiError, http::response::Body,
        user::recorded_video::RecordedVideoUpdate,
    },
    relay::state::RelayState,
};
use actix_web::{delete, get, patch, post, web, HttpResponse};

fn check_stream_owner(pool: &DbPool, stream: &str, owner: i32) -> Result<(), ApiError> {
    let source = find_stream_by_uuid(&mut *pool.get()?, stream.into())?;
    if source.user_id != owner {
        return Err(ApiError::Forbidden {
            error_message: "Only the stream owner can manage its videos".into(),
        });
    }
    Ok(())
}

#[delete("/{uuid}")]
pub async fn delete_video(
    pool: web::Data<DbPool>,
    state: web::Data<RelayState>,
    path: web::Path<String>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let uuid = path.into_inner();
    uuid::Uuid::parse_str(&uuid).map_err(|_| ApiError::BadRequest {
        error_message: "Invalid replay UUID".into(),
    })?;
    let video = recorded_videos::find(&mut *pool.get()?, &uuid)?;
    video.require_owner(session.principal.id)?;
    let _operation = state
        .media
        .recording_operation(&video.source_stream_uuid)
        .await;
    recorded_videos::prepare_delete(&mut *pool.get()?, &uuid, session.principal.id)?;
    if let Err(error) = state.media.delete_recording_files(&uuid).await {
        recorded_videos::deletion_failed(&mut *pool.get()?, &uuid, &error.to_string())?;
        return Err(ApiError::ServiceUnavailable {
            error_message: "Replay files could not be fully deleted. Retry permanent deletion."
                .into(),
        });
    }
    recorded_videos::finish_delete(&mut *pool.get()?, &uuid, session.principal.id)?;
    Ok(HttpResponse::NoContent().finish())
}

#[get("")]
pub async fn list_owned_videos(
    pool: web::Data<DbPool>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let videos = recorded_videos::list_owned(&mut *pool.get()?, session.principal.id)?;
    Ok(HttpResponse::Ok().json(Body::new("ok", videos)))
}
#[get("/{uuid}")]
pub async fn get_video(
    pool: web::Data<DbPool>,
    path: web::Path<String>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let video = recorded_videos::find(&mut *pool.get()?, &path)?;
    video.require_owner(session.principal.id)?;
    Ok(HttpResponse::Ok().json(Body::new("ok", video)))
}
#[get("/{uuid}")]
pub async fn get_public_video(
    pool: web::Data<DbPool>,
    path: web::Path<String>,
    session: Option<web::ReqData<Session>>,
) -> Result<HttpResponse, ApiError> {
    let video = recorded_videos::find(&mut *pool.get()?, &path)?;
    if !video.is_visible_to(session.as_ref().map(|s| s.principal.id)) {
        return Err(ApiError::NotFound {
            error_message: "Video is not available".into(),
        });
    }
    Ok(HttpResponse::Ok().json(Body::new("ok", video)))
}
#[get("/{stream}/videos")]
pub async fn list_videos(
    pool: web::Data<DbPool>,
    path: web::Path<String>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let stream = path.into_inner();
    check_stream_owner(&pool, &stream, session.principal.id)?;
    let videos = recorded_videos::list(&mut *pool.get()?, &stream, session.principal.id)?;
    Ok(HttpResponse::Ok().json(Body::new("ok", videos)))
}
/// Create metadata only, without starting a recording.
#[post("/{stream}/videos")]
pub async fn create_video(
    pool: web::Data<DbPool>,
    path: web::Path<String>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let video = recorded_videos::create(&mut *pool.get()?, &path, session.principal.id)?;
    Ok(HttpResponse::Created()
        .append_header(("Location", format!("/api/videos/{}", video.uuid)))
        .json(Body::new("ok", video)))
}
#[patch("/{uuid}")]
pub async fn update_video(
    pool: web::Data<DbPool>,
    path: web::Path<String>,
    session: web::ReqData<Session>,
    body: web::Json<RecordedVideoUpdate>,
) -> Result<HttpResponse, ApiError> {
    let video = recorded_videos::update(&mut *pool.get()?, &path, session.principal.id, &body)?;
    Ok(HttpResponse::Ok().json(Body::new("ok", video)))
}

#[get("/{stream}/recordings/ready")]
pub async fn recording_ready(
    pool: web::Data<DbPool>,
    state: web::Data<RelayState>,
    path: web::Path<String>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let stream = path.into_inner();
    uuid::Uuid::parse_str(&stream).map_err(|_| ApiError::BadRequest {
        error_message: "Invalid stream UUID".into(),
    })?;
    check_stream_owner(&pool, &stream, session.principal.id)?;
    let channel = {
        let mut conn = pool.get()?;
        let source = find_stream_by_uuid(&mut conn, stream.clone())?;
        crate::db::channels::read::find_channel_by_id(&mut conn, source.channel_id)?
    };
    let grant = moq_auth::Grant::new(
        Default::default(),
        [stream.parse().expect("validated stream UUID")]
            .into_iter()
            .collect(),
    );
    let token = moq_relay::auth::Token::new(&format!("live/{}", channel.uuid), &grant);
    let origin = state
        .cluster
        .subscriber(&token)
        .ok_or_else(|| ApiError::Forbidden {
            error_message: "Recording source is not authorized".into(),
        })?;
    state
        .media
        .wait_recording_ready(moq_mux::Source::new(origin.consume(), stream))
        .await
        .map_err(|error| ApiError::ServiceUnavailable {
            error_message: error.to_string(),
        })?;
    Ok(HttpResponse::NoContent()
        .append_header(("Cache-Control", "no-store"))
        .finish())
}

#[post("/{stream}/recordings")]
pub async fn start_recording(
    pool: web::Data<DbPool>,
    state: web::Data<RelayState>,
    path: web::Path<String>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let stream = path.into_inner();
    uuid::Uuid::parse_str(&stream).map_err(|_| ApiError::BadRequest {
        error_message: "Invalid stream UUID".into(),
    })?;
    check_stream_owner(&pool, &stream, session.principal.id)?;
    let _operation = state.media.recording_operation(&stream).await;
    if recorded_videos::list(&mut *pool.get()?, &stream, session.principal.id)?
        .iter()
        .any(|v| v.status == "recording")
    {
        return Err(ApiError::Conflict {
            error_message: "A recording is already running".into(),
        });
    }
    let video = recorded_videos::create(&mut *pool.get()?, &stream, session.principal.id)?;
    // Resolve through the embedded relay's normal, permission-scoped subscriber view.
    let grant = moq_auth::Grant::new(
        Default::default(),
        [stream.parse().expect("validated stream UUID")]
            .into_iter()
            .collect(),
    );
    let token = moq_relay::auth::Token::new(&format!("live/{}", video.channel_uuid), &grant);
    let source = state
        .cluster
        .subscriber(&token)
        .map(|origin| moq_mux::Source::new(origin.consume(), stream.clone()));
    let started = match source {
        Some(source) => {
            state
                .media
                .start_recording(&stream, &video.uuid, source, pool.get_ref().clone())
                .await
        }
        None => Err(anyhow::anyhow!("Recording source is not authorized")),
    };
    if let Err(error) = started {
        let video =
            recorded_videos::recording_failed(&mut *pool.get()?, &video.uuid, &error.to_string())?;
        return Ok(HttpResponse::ServiceUnavailable().json(Body::new(&error.to_string(), video)));
    }
    let video = match recorded_videos::recording_started(&mut *pool.get()?, &video.uuid) {
        Ok(video) => video,
        Err(error) => {
            let _ = state.media.stop(&stream).await;
            recorded_videos::recording_failed(
                &mut *pool.get()?,
                &video.uuid,
                "Cannot persist recording start",
            )?;
            return Err(error);
        }
    };
    Ok(HttpResponse::Created()
        .append_header(("Location", format!("/api/videos/{}", video.uuid)))
        .json(Body::new("ok", video)))
}
#[post("/{uuid}/stop")]
pub async fn stop_recording(
    pool: web::Data<DbPool>,
    state: web::Data<RelayState>,
    path: web::Path<String>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let uuid = path.into_inner();
    let video = recorded_videos::find(&mut *pool.get()?, &uuid)?;
    video.require_owner(session.principal.id)?;
    let _operation = state
        .media
        .recording_operation(&video.source_stream_uuid)
        .await;
    let video = recorded_videos::find(&mut *pool.get()?, &uuid)?;
    if video.status != "recording" {
        return Ok(HttpResponse::Ok().json(Body::new("ok", video)));
    }
    if let Err(error) = state.media.stop(&video.source_stream_uuid).await {
        let video =
            recorded_videos::recording_failed(&mut *pool.get()?, &uuid, &error.to_string())?;
        return Ok(HttpResponse::ServiceUnavailable().json(Body::new(&error.to_string(), video)));
    }
    // Stop waits for the HLS writer and its persisted ready/failed result.
    let video = recorded_videos::find(&mut *pool.get()?, &uuid)?;
    Ok(HttpResponse::Ok().json(Body::new("ok", video)))
}
