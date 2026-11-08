use crate::db::{devices, DbPool};
use crate::models::auth::{jwt::JWTConfig, session::Session};
use crate::models::device::{grant, DeviceView};
use crate::models::error::ApiError;
use crate::models::http::response::Body;
use actix_web::{delete, get, patch, post, web, HttpResponse};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct DeviceName {
    name: String,
}

#[derive(Deserialize)]
pub struct DeviceUpdate {
    name: Option<String>,
    active: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InviteRequest {
    stream_uuid: String,
}

#[derive(Serialize)]
struct Invite {
    token: String,
}

#[derive(Deserialize)]
pub struct RedeemRequest {
    token: String,
}

fn device_uuid(value: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(value).map_err(|_| ApiError::NotFound {
        error_message: "Device not found".into(),
    })
}

#[get("")]
pub async fn list(
    pool: web::Data<DbPool>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let devices = devices::read::list(&mut *pool.get()?, session.principal.id)?
        .into_iter()
        .map(DeviceView::from)
        .collect::<Vec<_>>();
    Ok(HttpResponse::Ok().json(Body::new("ok", devices)))
}

#[post("")]
pub async fn create(
    pool: web::Data<DbPool>,
    session: web::ReqData<Session>,
    body: web::Json<DeviceName>,
) -> Result<HttpResponse, ApiError> {
    let device = devices::create::create(&mut *pool.get()?, session.principal.id, &body.name)?;
    Ok(HttpResponse::Created().json(Body::new("ok", DeviceView::from(device))))
}

#[patch("/{uuid}")]
pub async fn update(
    pool: web::Data<DbPool>,
    session: web::ReqData<Session>,
    path: web::Path<String>,
    body: web::Json<DeviceUpdate>,
) -> Result<HttpResponse, ApiError> {
    let device = devices::update::update(
        &mut *pool.get()?,
        session.principal.id,
        device_uuid(&path)?,
        body.name.as_deref(),
        body.active,
    )?;
    Ok(HttpResponse::Ok().json(Body::new("ok", DeviceView::from(device))))
}

#[delete("/{uuid}")]
pub async fn remove(
    pool: web::Data<DbPool>,
    session: web::ReqData<Session>,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    devices::delete::delete(&mut *pool.get()?, session.principal.id, device_uuid(&path)?)?;
    Ok(HttpResponse::NoContent().finish())
}

#[post("/{uuid}/invite")]
pub async fn invite(
    pool: web::Data<DbPool>,
    session: web::ReqData<Session>,
    path: web::Path<String>,
    body: web::Json<InviteRequest>,
) -> Result<HttpResponse, ApiError> {
    let token = grant::issue_invite(
        &mut *pool.get()?,
        session.principal.id,
        device_uuid(&path)?,
        &body.stream_uuid,
    )?;
    Ok(HttpResponse::Created()
        .insert_header(("Cache-Control", "no-store"))
        .json(Body::new("ok", Invite { token })))
}

#[post("/device/redeem")]
pub async fn redeem(
    pool: web::Data<DbPool>,
    config: web::Data<JWTConfig>,
    body: web::Json<RedeemRequest>,
) -> Result<HttpResponse, ApiError> {
    let session = grant::redeem(&mut *pool.get()?, &body.token, config.session_live_time)?;
    Ok(HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-store"))
        .json(Body::new("ok", session)))
}

#[post("/device/disconnect")]
pub async fn disconnect(
    pool: web::Data<DbPool>,
    body: web::Json<RedeemRequest>,
) -> Result<HttpResponse, ApiError> {
    grant::end_session(&mut *pool.get()?, &body.token)?;
    Ok(HttpResponse::NoContent().finish())
}

