use crate::db::DbPool;
use crate::files::FilesConfig;
use crate::models::auth::session::Session;
use crate::models::error::ApiError;
use crate::models::http::response::Body;
use crate::models::http::MESSAGE_OK;
use crate::models::user::channel::{Channel, ChannelForm};
use actix_multipart::form::MultipartForm;
use actix_web::{get, post, put, web, HttpResponse};

// PUT api/channel
#[put("")]
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
#[get("/{uuid}")]
pub async fn get_channel(
    pool: web::Data<DbPool>,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let chan =  Channel::fetch(&pool, path.into_inner())?;
    Ok(HttpResponse::Ok().json(Body::new("ok", chan)))
}

#[get("/streaming-policy")]
pub async fn get_streaming_policy(
    pool: web::Data<DbPool>, session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let info = crate::models::streaming_policy::info(&pool, &session.principal)?;
    Ok(HttpResponse::Ok().insert_header(("Cache-Control", "no-store")).json(Body::new("ok", info)))
}

#[post("/streaming-request")]
pub async fn request_streaming(
    pool: web::Data<DbPool>, session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let mut conn = pool.get()?;
    let created = crate::models::streaming_policy::request(&mut conn, session.principal.channel_id)?;
    if created {
        crate::models::inbox::streaming_permission_requested(
            &mut conn,
            session.principal.id,
            &session.principal.name,
            &session.principal.channel_uuid,
        )?;
    }
    Ok(HttpResponse::Ok().json(Body::new("ok", "requested")))
}
