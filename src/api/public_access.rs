use crate::{db::DbPool, models::{error::ApiError, http::response::Body, streaming_policy}};
use actix_web::{get, web, HttpResponse};

#[get("/public-content")]
pub async fn public_content(pool: web::Data<DbPool>) -> Result<HttpResponse, ApiError> {
    let mut conn = pool.get()?;
    let enabled = streaming_policy::public_content_enabled(&mut conn)?;
    Ok(HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-store"))
        .json(Body::new("ok", enabled)))
}
