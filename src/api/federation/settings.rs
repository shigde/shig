use crate::federation::FederationConfig;
use crate::models::error::ApiError;
use crate::models::federation::settings::Settings;
use crate::models::http::response::Body;
use crate::models::http::MESSAGE_OK;
use actix_web::{get, web, HttpResponse};

#[get("/settings")]
pub async fn get_settings(cfg: web::Data<FederationConfig>) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(Body::new(MESSAGE_OK, Settings::new(&cfg))))
}
