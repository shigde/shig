use actix_multipart::form::MultipartForm;
use actix_web::{delete, get, post, put, web, HttpResponse};
use crate::db::DbPool;
use crate::files::FilesConfig;
use crate::models::auth::session::Session;
use crate::models::error::ApiError;
use crate::models::http::MESSAGE_OK;
use crate::models::http::response::Body;
use crate::models::user::stream::Stream;
use crate::models::user::stream_form::StreamForm;

// GET api/user/stream/:id
#[get("/{uuid}")]
pub async fn get_stream(
    pool: web::Data<DbPool>,
    path: web::Path<String>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let stream =  Stream::fetch(&pool, path.into_inner(), session.principal.clone())?;
    Ok(HttpResponse::Ok().json(Body::new("ok", stream)))
}

#[delete("/{uuid}")]
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

// POST api/user/stream
#[post("")]
pub async fn create_stream(
    pool: web::Data<DbPool>,
    MultipartForm(form): MultipartForm<StreamForm>,
    session: web::ReqData<Session>,
    cfg: web::Data<FilesConfig>,
) -> Result<HttpResponse, ApiError> {
    match form.save(&pool, session.principal.clone(), &cfg) {
        Ok(stream) => Ok(HttpResponse::Ok().json(Body::new(MESSAGE_OK, stream))),
        Err(err) => Err(err),
    }
}

// PUT api/user/stream
#[put("")]
pub async fn update_stream(
    pool: web::Data<DbPool>,
    MultipartForm(form): MultipartForm<StreamForm>,
    session: web::ReqData<Session>,
    cfg: web::Data<FilesConfig>,
) -> Result<HttpResponse, ApiError> {
    match form.update(&pool, session.principal.clone(), &cfg) {
        Ok(stream) => Ok(HttpResponse::Ok().json(Body::new(MESSAGE_OK, stream))),
        Err(err) => Err(err),
    }
}
