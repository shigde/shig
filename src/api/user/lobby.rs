use crate::db::DbPool;
use crate::models::auth::session::Session;
use crate::models::error::ApiError;
use crate::models::http::response::Body;
use crate::models::http::MESSAGE_OK;
use crate::models::lobby;
use crate::models::lobby::{fetch_all_participants, is_lobby_online, leave_lobby};
use actix_web::{delete, get, post, web, HttpRequest, HttpResponse};

#[get("/{channel_uuid}/stream/{stream_uuid}/lobby/online")]
pub async fn is_online(
    _req: HttpRequest,
    pool: web::Data<DbPool>,
    path: web::Path<(String, String)>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let (channel_uuid, stream_uuid) = path.into_inner();
    let user = session.principal.clone();
    let status = is_lobby_online(&pool, channel_uuid, stream_uuid, user).await?;

    Ok(HttpResponse::Ok().json(Body::new(MESSAGE_OK, status)))
}

#[get("/{channel_uuid}/stream/{stream_uuid}/lobby/participants")]
pub async fn participants_list(
    pool: web::Data<DbPool>,
    path: web::Path<(String, String)>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let (channel_uuid, stream_uuid) = path.into_inner();
    let user = session.principal.clone();
    let participants = fetch_all_participants(&pool, channel_uuid, stream_uuid, user).await?;
    Ok(HttpResponse::Ok().json(Body::new(MESSAGE_OK, participants)))
}

#[delete("/{channel_uuid}/stream/{stream_uuid}/lobby/leave")]
pub async fn participant_leave(
    pool: web::Data<DbPool>,
    sfu_addr: web::Data<actix::Addr<crate::sfu::Sfu>>,
    path: web::Path<(String, String)>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let (channel_uuid, stream_uuid) = path.into_inner();
    let user = session.principal.clone();
    leave_lobby(&pool, channel_uuid, stream_uuid, user, sfu_addr).await?;
    Ok(HttpResponse::Ok().json(Body::new(MESSAGE_OK, {})))
}

#[post("/{channel_uuid}/stream/{stream_uuid}/lobby/live")]
pub async fn start_streaming(
    pool: web::Data<DbPool>,
    sfu_addr: web::Data<actix::Addr<crate::sfu::Sfu>>,
    path: web::Path<(String, String)>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let (channel_uuid, stream_uuid) = path.into_inner();
    let user = session.principal.clone();
    lobby::streaming::publish(&pool, channel_uuid, stream_uuid, user, sfu_addr, true).await?;

    Ok(HttpResponse::Ok().json(Body::new(MESSAGE_OK, {})))
}

#[delete("/{channel_uuid}/stream/{stream_uuid}/lobby/live")]
pub async fn stop_streaming(
    pool: web::Data<DbPool>,
    sfu_addr: web::Data<actix::Addr<crate::sfu::Sfu>>,
    path: web::Path<(String, String)>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let (channel_uuid, stream_uuid) = path.into_inner();
    let user = session.principal.clone();
    lobby::streaming::publish(&pool, channel_uuid, stream_uuid, user, sfu_addr, false).await?;
    Ok(HttpResponse::Ok().json(Body::new(MESSAGE_OK, {})))
}
