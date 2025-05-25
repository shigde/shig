use actix_multipart::form::MultipartForm;
use actix_web::{web, HttpResponse};
use crate::db::DbPool;
use crate::files::FilesConfig;
use crate::models::auth::session::Session;
use crate::models::error::ApiError;
use crate::models::http::MESSAGE_OK;
use crate::models::http::response::Body;
use crate::models::user::stream::Stream;
use crate::models::user::stream_form::StreamForm;

pub async fn get_stream(
    pool: web::Data<DbPool>,
    path: web::Path<String>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let stream =  Stream::fetch(&pool, path.into_inner(), session.principal.clone())?;
    Ok(HttpResponse::Ok().json(Body::new("ok", stream)))
}

// DELETE api/user/stream/:id
pub async fn delete_stream(
    pool: web::Data<DbPool>,
    path: web::Path<String>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    match Stream::delete(&pool, path.into_inner(), session.principal.clone()) {
        Ok(channel) => Ok(HttpResponse::Ok().json(Body::new(MESSAGE_OK, channel))),
        Err(err) => Err(err),
    }
}

// Get api/user/stream/:id
pub async fn create_stream(
    pool: web::Data<DbPool>,
    MultipartForm(form): MultipartForm<StreamForm>,
    session: web::ReqData<Session>,
    cfg: web::Data<FilesConfig>,
) -> Result<HttpResponse, ApiError> {
    match form.save(&pool, session.principal.clone(), &cfg) {
        Ok(channel) => Ok(HttpResponse::Ok().json(Body::new(MESSAGE_OK, channel))),
        Err(err) => Err(err),
    }
}

// PUT api/user/stream/:id
pub async fn update_stream(
    pool: web::Data<DbPool>,
    MultipartForm(form): MultipartForm<StreamForm>,
    session: web::ReqData<Session>,
    cfg: web::Data<FilesConfig>,
) -> Result<HttpResponse, ApiError> {
    match form.update(&pool, session.principal.clone(), &cfg) {
        Ok(channel) => Ok(HttpResponse::Ok().json(Body::new(MESSAGE_OK, channel))),
        Err(err) => Err(err),
    }
}
