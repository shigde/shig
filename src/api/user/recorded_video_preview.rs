use crate::db::DbPool;
use crate::models::auth::session::Session;
use crate::models::error::ApiError;
use crate::models::http::response::Body;
use crate::models::user::recorded_video_preview::RecordedVideoPreview;
use actix_web::{get, web, HttpResponse};

#[get("/{uuid}")]
pub async fn get_recorded_video_preview(
    pool: web::Data<DbPool>,
    path: web::Path<String>,
    session: Option<web::ReqData<Session>>,
) -> Result<HttpResponse, ApiError> {
    let stream = RecordedVideoPreview::fetch(&pool, path.into_inner(), session.is_some())?;
    Ok(HttpResponse::Ok().json(Body::new("ok", stream)))
}

#[get("")]
pub async fn get_recorded_video_preview_list(
    pool: web::Data<DbPool>,
    session: Option<web::ReqData<Session>>,
) -> Result<HttpResponse, ApiError> {
    let stream_list = RecordedVideoPreview::fetch_all(&pool, session.is_some())?;
    Ok(HttpResponse::Ok().json(Body::new("ok", stream_list)))
}

#[get("/{channel_uuid}/recorded-video-preview")]
pub async fn get_channel_recorded_video_preview_list(
    pool: web::Data<DbPool>,
    path: web::Path<String>,
    session: Option<web::ReqData<Session>>,
) -> Result<HttpResponse, ApiError> {
    let stream_list =
        RecordedVideoPreview::fetch_all_by_channel(&pool, path.into_inner(), session.is_some())?;
    Ok(HttpResponse::Ok().json(Body::new("ok", stream_list)))
}
