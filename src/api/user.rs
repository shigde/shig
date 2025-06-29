use actix_web::{get, web, HttpResponse};
use crate::db::DbPool;
use crate::models::error::ApiError;
use crate::models::http::response::Body;
use crate::models::user::User;

pub mod channel;
pub mod stream;
pub mod stream_preview;

// GET api/pup/user/:user_uuid
#[get("/{user_uuid}")]
pub async fn get_active_user(
    pool: web::Data<DbPool>,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let user =  User::find_as_active(&pool, path.into_inner().as_str())?;
    Ok(HttpResponse::Ok().json(Body::new("ok", user)))
}
