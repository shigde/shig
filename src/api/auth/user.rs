use crate::models::auth::session::Session;
use crate::models::error::ApiError;
use crate::models::http::response::Body;
use crate::models::user::User;
use actix_web::{web, HttpResponse};

// GET api/auth/user
pub async fn get_current_user(session: web::ReqData<Session>) -> Result<HttpResponse, ApiError> {
    let current_user = User::from_principal(session.principal.clone());
    Ok(HttpResponse::Ok().json(Body::new("ok", current_user)))
}
