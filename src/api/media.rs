use crate::{
    db::{streams::read::find_stream_by_uuid, DbPool},
    models::{auth::session::Session, error::ApiError, http::response::Body},
    relay::state::RelayState,
};
use actix_web::{delete, get, post, put, web, HttpResponse};
use serde::Deserialize;

fn failure(error: anyhow::Error) -> ApiError {
    log::warn!("Media operation: {error:#}");
    ApiError::Conflict {
        error_message: error.to_string(),
    }
}
fn owner(pool: &DbPool, stream: &str, session: &Session) -> Result<(), ApiError> {
    let mut conn = pool.get()?;
    let record = find_stream_by_uuid(&mut conn, stream.to_string())?;
    if record.user_id != session.principal.id {
        return Err(ApiError::Forbidden {
            error_message: "Only the stream owner can manage recordings".into(),
        });
    }
    Ok(())
}
/// Same visibility as live viewing, plus owner-only unpublished previews.
async fn viewer(
    pool: &DbPool,
    state: &RelayState,
    stream: &str,
    session: Option<&Session>,
) -> Result<(), ApiError> {
    let (is_public, user_id) = {
        let mut conn = pool.get()?;
        let record = find_stream_by_uuid(&mut conn, stream.to_string())?;
        (record.is_public, record.user_id)
    };
    let preview = session
        .filter(|s| s.principal.id == user_id)
        .map(|s| s.principal.id);
    let recording = state.media.status(stream).await.map_err(failure)?;
    if !is_public && session.is_none() {
        return Err(ApiError::Unauthorized {
            error_message: "Login required".into(),
        });
    }
    if recording.status != "ready" || (!recording.published && preview.is_none()) {
        return Err(ApiError::NotFound {
            error_message: "Recording is not available".into(),
        });
    }
    Ok(())
}
#[get("/{stream}/recording")]
pub async fn status(
    pool: web::Data<DbPool>,
    state: web::Data<RelayState>,
    path: web::Path<String>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let stream = path.into_inner();
    owner(&pool, &stream, &session)?;
    Ok(HttpResponse::Ok().json(Body::new(
        "ok",
        state.media.status(&stream).await.map_err(failure)?,
    )))
}
#[post("/{stream}/recording")]
pub async fn start(
    pool: web::Data<DbPool>,
    state: web::Data<RelayState>,
    path: web::Path<String>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let stream = path.into_inner();
    owner(&pool, &stream, &session)?;
    Ok(HttpResponse::Ok().json(Body::new(
        "ok",
        state.media.start(&stream).await.map_err(failure)?,
    )))
}
#[delete("/{stream}/recording")]
pub async fn stop(
    pool: web::Data<DbPool>,
    state: web::Data<RelayState>,
    path: web::Path<String>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let stream = path.into_inner();
    owner(&pool, &stream, &session)?;
    Ok(HttpResponse::Ok().json(Body::new(
        "ok",
        state.media.stop(&stream).await.map_err(failure)?,
    )))
}
#[derive(Deserialize)]
pub struct Publication {
    published: bool,
}
#[put("/{stream}/recording/publication")]
pub async fn publication(
    pool: web::Data<DbPool>,
    state: web::Data<RelayState>,
    path: web::Path<String>,
    session: web::ReqData<Session>,
    body: web::Json<Publication>,
) -> Result<HttpResponse, ApiError> {
    let stream = path.into_inner();
    owner(&pool, &stream, &session)?;
    Ok(HttpResponse::Ok().json(Body::new(
        "ok",
        state
            .media
            .publish(&stream, body.published)
            .await
            .map_err(failure)?,
    )))
}
#[get("/{stream}/recording")]
pub async fn available(
    pool: web::Data<DbPool>,
    state: web::Data<RelayState>,
    path: web::Path<String>,
    session: Option<web::ReqData<Session>>,
) -> Result<HttpResponse, ApiError> {
    let stream = path.into_inner();
    match viewer(&pool, &state, &stream, session.as_deref()).await {
        Ok(_) => Ok(HttpResponse::Ok().json(Body::new(
            "ok",
            Some(state.media.status(&stream).await.map_err(failure)?),
        ))),
        Err(ApiError::NotFound { .. }) => Ok(HttpResponse::Ok().json(Body::new(
            "ok",
            Option::<crate::sfu::media::Recording>::None,
        ))),
        Err(error) => Err(error),
    }
}
