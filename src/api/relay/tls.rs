use crate::relay::state::RelayState;
use actix_web::{get, web, HttpResponse};

#[get("/certificate.sha256")]
pub async fn get_fingerprint(state: web::Data<RelayState>) -> actix_web::Result<HttpResponse> {
    state.forward_http("/certificate.sha256".to_string()).await
}
