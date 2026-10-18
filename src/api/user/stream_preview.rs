use crate::db::DbPool;
use crate::models::auth::session::Session;
use crate::models::error::ApiError;
use crate::models::http::response::Body;
use crate::models::user::stream_preview::StreamPreview;
use actix_web::{get, web, HttpResponse};

#[get("/{uuid}")]
pub async fn get_stream_preview(
    pool: web::Data<DbPool>,
    path: web::Path<String>,
    session: Option<web::ReqData<Session>>,
) -> Result<HttpResponse, ApiError> {
    let stream = StreamPreview::fetch(&pool, path.into_inner(), session.is_some())?;
    Ok(HttpResponse::Ok().json(Body::new("ok", stream)))
}

#[get("")]
pub async fn get_stream_preview_list(
    pool: web::Data<DbPool>,
    session: Option<web::ReqData<Session>>,
) -> Result<HttpResponse, ApiError> {
    let stream_list = StreamPreview::fetch_all(&pool, session.is_some())?;
    Ok(HttpResponse::Ok().json(Body::new("ok", stream_list)))
}

#[get("/{channel_uuid}/stream-preview")]
pub async fn get_channel_stream_preview_list(
    pool: web::Data<DbPool>,
    path: web::Path<String>,
    session: Option<web::ReqData<Session>>,
) -> Result<HttpResponse, ApiError> {
    let stream_list =
        StreamPreview::fetch_all_by_channel(&pool, path.into_inner(), session.is_some())?;
    Ok(HttpResponse::Ok().json(Body::new("ok", stream_list)))
}
