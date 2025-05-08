use crate::db::DbPool;
use crate::files::FilesConfig;
use crate::models::auth::session::Session;
use crate::models::error::ApiError;
use crate::models::http::response::Body;
use crate::models::http::MESSAGE_OK;
use crate::models::user::channel::{Channel, ChannelForm};
use actix_multipart::form::MultipartForm;
use actix_web::{web, HttpResponse};

// PUT api/user/channel/:id
pub async fn update_channel(
    pool: web::Data<DbPool>,
    MultipartForm(form): MultipartForm<ChannelForm>,
    session: web::ReqData<Session>,
    cfg: web::Data<FilesConfig>,
) -> Result<HttpResponse, ApiError> {
    match form.save(&pool, session.principal.clone(), &cfg) {
        Ok(channel) => Ok(HttpResponse::Ok().json(Body::new(MESSAGE_OK, channel))),
        Err(err) => Err(err),
    }
}

// Get api/pub/channel/:uuid
pub async fn get_channel(
    pool: web::Data<DbPool>,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let chan =  Channel::fetch(&pool, path.into_inner())?;
    Ok(HttpResponse::Ok().json(Body::new("ok", chan)))
}
