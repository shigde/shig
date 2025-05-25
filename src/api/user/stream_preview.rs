use actix_web::{web, HttpResponse};
use crate::db::DbPool;
use crate::models::error::ApiError;
use crate::models::http::response::Body;
use crate::models::user::stream_preview::StreamPreview;

pub async fn get_stream_preview(
    pool: web::Data<DbPool>,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let stream =  StreamPreview::fetch(&pool, path.into_inner())?;
    Ok(HttpResponse::Ok().json(Body::new("ok", stream)))
}

pub async fn get_stream_preview_list(
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, ApiError> {
    let stream_list =  StreamPreview::fetch_all(&pool)?;
    Ok(HttpResponse::Ok().json(Body::new("ok", stream_list)))
}