use actix_web::{web, HttpResponse};

use crate::db::DbPool;

use crate::models::auth::verify::Verify;
use crate::models::error::ApiError;

// GET api/auth/verify/{token}
pub async fn verify(
    pool: web::Data<DbPool>,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    match Verify::user(&pool, path) {
        Ok(()) => Ok(HttpResponse::Ok().json("")),
        Err(err) => Err(err),
    }
}
