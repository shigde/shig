use crate::{
    db::DbPool,
    models::{auth::session::Session, error::ApiError, http::response::Body, reactions},
};
use actix_web::{post, web, HttpResponse};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LikeUpdate {
    liked: bool,
}

#[post("/{kind}/{uuid}")]
pub async fn set_like(
    pool: web::Data<DbPool>,
    path: web::Path<(String, String)>,
    session: web::ReqData<Session>,
    body: web::Json<LikeUpdate>,
) -> Result<HttpResponse, ApiError> {
    let (kind, uuid) = path.into_inner();
    uuid::Uuid::parse_str(&uuid).map_err(|_| ApiError::BadRequest {
        error_message: "Invalid content UUID".into(),
    })?;
    let mut conn = pool.get()?;
    let stats = reactions::set(&mut conn, &kind, &uuid, session.principal.id, body.liked)?;
    Ok(HttpResponse::Ok().json(Body::new("ok", stats)))
}
