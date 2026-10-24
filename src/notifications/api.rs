use crate::{
    db::DbPool,
    inbox::{self, Inbox},
    models::{auth::session::Session, error::ApiError, http::response::Body},
};
use actix_web::{get, patch, web, HttpResponse};
use serde::Deserialize;
const INBOX: Inbox = Inbox::Notifications;
#[derive(Deserialize)]
pub struct Query {
    #[serde(default)]
    offset: i64,
}
#[get("")]
pub async fn list(
    pool: web::Data<DbPool>,
    session: web::ReqData<Session>,
    query: web::Query<Query>,
) -> Result<HttpResponse, ApiError> {
    if query.offset < 0 {
        return Err(ApiError::BadRequest {
            error_message: "Invalid offset".into(),
        });
    }
    let data = inbox::list(&mut *pool.get()?, INBOX, session.principal.id, query.offset)?;
    Ok(HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-store"))
        .json(Body::new("ok", data)))
}
#[get("/counts")]
pub async fn counts(
    pool: web::Data<DbPool>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let data = inbox::counts(&mut *pool.get()?, INBOX, session.principal.id)?;
    Ok(HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-store"))
        .json(Body::new("ok", data)))
}
#[patch("/read")]
pub async fn read_all(
    pool: web::Data<DbPool>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    inbox::mark_all_read(&mut *pool.get()?, INBOX, session.principal.id)?;
    Ok(HttpResponse::Ok().json(Body::new("ok", true)))
}
#[patch("/{id}/read")]
pub async fn read(
    pool: web::Data<DbPool>,
    session: web::ReqData<Session>,
    id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    if !inbox::mark_read(&mut *pool.get()?, INBOX, session.principal.id, &id)? {
        return Err(ApiError::NotFound {
            error_message: "Entry not found".into(),
        });
    }
    Ok(HttpResponse::Ok().json(Body::new("ok", true)))
}
