use crate::db::DbPool;
use crate::models::auth::pass::{ForgottenPassword, ResetPassword, UpdatePassword};
use crate::models::auth::session::Session;
use crate::models::error::ApiError;
use crate::models::http::response::Body;
use crate::models::http::MESSAGE_OK;
use crate::models::mail::config::MailConfig;
use actix_web::{web, HttpResponse};

// POST api/auth/pass/email
pub async fn send_forgotten_pass_email(
    pool: web::Data<DbPool>,
    email_dto: web::Json<ForgottenPassword>,
    cfg: web::Data<MailConfig>,
) -> Result<HttpResponse, ApiError> {
    match email_dto.send_forgotten_password_email(&pool, &cfg).await {
        _ => Ok(HttpResponse::Ok().json({})),
    }
}

// PUT api/auth/pass/reset
pub async fn reset_password(
    pool: web::Data<DbPool>,
    reset_dto: web::Json<ResetPassword>,
) -> Result<HttpResponse, ApiError> {
    let success = reset_dto.set_new_passwort(&pool).is_ok();

    if !success {
        return Ok(HttpResponse::Conflict().json({}));
    }
    Ok(HttpResponse::Ok().json(Body::new(MESSAGE_OK, {})))
}

// PUT api/auth/pass/update
pub async fn update_password(
    pool: web::Data<DbPool>,
    update_dto: web::Json<UpdatePassword>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let success = update_dto
        .set_new_passwort(&pool, session.principal.clone())
        .is_ok();

    if !success {
        return Ok(HttpResponse::Conflict().json({}));
    }
    Ok(HttpResponse::Ok().json(Body::new(MESSAGE_OK, {})))
}
