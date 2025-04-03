use crate::db::DbPool;
use crate::models::auth::pass::ForgottenPassword;
use crate::models::auth::session::Session;
use crate::models::error::ApiError;
use crate::models::http::response::Body;
use crate::models::http::MESSAGE_OK;
use crate::models::mail::config::MailConfig;
use crate::models::user::channel::ChannelForm;
use actix_multipart::form::MultipartForm;
use actix_web::{web, HttpResponse};
use crate::files::FilesConfig;

// PUT api/user/channel/:id
pub async fn update_channel(
    pool: web::Data<DbPool>,
    MultipartForm(form): MultipartForm<ChannelForm>,
    session: web::ReqData<Session>,
    cfg: web::Data<FilesConfig>,
) -> Result<HttpResponse, ApiError> {
    match form.save(&pool, session.principal.clone(), &cfg) {
        Ok(channel) => Ok(HttpResponse::Ok().json(Body::new(MESSAGE_OK, channel))),
        Err(err) => Err(err),
    }
}

// Get api/user/channel/:id
pub async fn get_channel(
    pool: web::Data<DbPool>,
    email_dto: web::Json<ForgottenPassword>,
    cfg: web::Data<MailConfig>,
) -> Result<HttpResponse, ApiError> {
    match email_dto.send_forgotten_password_email(&pool, &cfg).await {
        _ => Ok(HttpResponse::Ok().json({})),
    }
}
