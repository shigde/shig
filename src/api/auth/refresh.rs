use crate::models::http::{MESSAGE_CREATED};
use actix_web::{web, HttpResponse};
use crate::db::DbPool;
use crate::models::auth::jwt::JWTConfig;
use crate::models::auth::refresh::Refresh;
use crate::models::error::ApiError;
use crate::models::http::response::Body;

// POST api/auth/refresh
pub async fn refresh(
    pool: web::Data<DbPool>,
    token_payload: web::Json<Refresh>,
    cfg: web::Data<JWTConfig>,
) -> Result<HttpResponse, ApiError> {
    match Refresh::generate_token(&pool, &token_payload, &cfg) {
        Ok(token_res) => Ok(HttpResponse::Created().json(Body::new(MESSAGE_CREATED, token_res))),
        Err(err) => Err(err),
    }
}
