use crate::{
    db::{
        inbox::{read as inbox_read, update, Inbox},
        DbPool,
    },
    models::{
        auth::session::Session,
        error::ApiError,
        http::response::Body,
        inbox::{Counts, Page},
    },
};
use actix_web::{get, patch, web, HttpResponse};
use serde::Deserialize;
const INBOX: Inbox = Inbox::Messages;
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
    let data = Page::from(inbox_read::list(
        &mut *pool.get()?,
        INBOX,
        session.principal.id,
        query.offset,
    )?);
    Ok(HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-store"))
        .json(Body::new("ok", data)))
}
#[get("/counts")]
pub async fn counts(
    pool: web::Data<DbPool>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let data = Counts::from(inbox_read::counts(
        &mut *pool.get()?,
        INBOX,
        session.principal.id,
    )?);
    Ok(HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-store"))
        .json(Body::new("ok", data)))
}
#[patch("/read")]
pub async fn read_all(
    pool: web::Data<DbPool>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    update::mark_all_read(&mut *pool.get()?, INBOX, session.principal.id)?;
    Ok(HttpResponse::Ok().json(Body::new("ok", true)))
}
#[patch("/{id}/read")]
pub async fn read(
    pool: web::Data<DbPool>,
    session: web::ReqData<Session>,
    id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    if !update::mark_read(&mut *pool.get()?, INBOX, session.principal.id, &id)? {
        return Err(ApiError::NotFound {
            error_message: "Entry not found".into(),
        });
    }
    Ok(HttpResponse::Ok().json(Body::new("ok", true)))
}
