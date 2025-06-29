use crate::db::users::USER_EMAIL_ALREADY_EXIST;
use crate::db::DbPool;
use crate::models::auth::signup::SingUp;
use crate::models::error::ApiError;
use crate::models::mail::config::MailConfig;
use actix_web::http::StatusCode;
use actix_web::{post, web, HttpResponse};

#[post("/register")]
pub async fn signup(
    pool: web::Data<DbPool>,
    sing_up_dto: web::Json<SingUp>,
    cfg: web::Data<MailConfig>,
) -> Result<HttpResponse, ApiError> {
    match SingUp::user(&pool, &sing_up_dto, &cfg).await {
        Ok(_) => Ok(HttpResponse::Ok().json("")),
        Err(err) => {
            if err.is_status_code(StatusCode::CONFLICT) && err.is_message(USER_EMAIL_ALREADY_EXIST)
            {
                return Ok(HttpResponse::Ok().json(""));
            }
            Err(err)
        }
    }
}
