use actix_web::{delete, get, web, HttpRequest, HttpResponse};
use crate::db::DbPool;
use crate::models::auth::session::Session;
use crate::models::error::ApiError;
use crate::models::http::MESSAGE_OK;
use crate::models::http::response::Body;


#[get("/{channel_uuid}/stream/{stream_uuid}/lobby/online")]
pub async fn is_online(
    _req: HttpRequest,
    _pool: web::Data<DbPool>,
    _sfu_addr: web::Data<actix::Addr<crate::sfu::Sfu>>,
    _path: web::Path<(String, String)>,
    _session: web::ReqData<Session>,
    _body: String,
) -> Result<HttpResponse, ApiError> {

    Ok(HttpResponse::Ok().json(Body::new(MESSAGE_OK, {})))
}

#[get("/{channel_uuid}/stream/{stream_uuid}/lobby/participants")]
pub async fn online_list(
    _req: HttpRequest,
    _pool: web::Data<DbPool>,
    _sfu_addr: web::Data<actix::Addr<crate::sfu::Sfu>>,
    _path: web::Path<(String, String)>,
    _session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {

    Ok(HttpResponse::Ok().json(Body::new(MESSAGE_OK, {})))
}

#[delete("/{channel_uuid}/stream/{stream_uuid}/lobby/leave")]
pub async fn leave(
    _req: HttpRequest,
    _pool: web::Data<DbPool>,
    _sfu_addr: web::Data<actix::Addr<crate::sfu::Sfu>>,
    _path: web::Path<(String, String)>,
    _session: web::ReqData<Session>,
    _body: String,
) -> Result<HttpResponse, ApiError> {

    Ok(HttpResponse::Ok().json(Body::new(MESSAGE_OK, {})))
}