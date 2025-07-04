use crate::db::DbPool;
use crate::models::auth::jwt::JWTConfig;
use crate::models::auth::login::Login;
use crate::models::error::ApiError;
use crate::models::http::response::Body;
use crate::models::http::{MESSAGE_LOGIN_FAILED, MESSAGE_LOGIN_SUCCESS};
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse};

#[post("/login")]
pub async fn login(
    pool: web::Data<DbPool>,
    login_dto: web::Json<Login>,
    cfg: web::Data<JWTConfig>,
) -> Result<HttpResponse, ApiError> {
    match Login::authenticate(&pool, &login_dto, &cfg) {
        Ok(token_res) => Ok(HttpResponse::Ok().json(Body::new(MESSAGE_LOGIN_SUCCESS, token_res))),
        Err(err) => {
            if err.is_status_code(StatusCode::NOT_FOUND) {
                return Ok(HttpResponse::build(StatusCode::FORBIDDEN)
                    .json(Body::new(MESSAGE_LOGIN_FAILED, {})))
            }
            Err(err)
        }
    }
}
