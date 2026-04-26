use crate::relay::state::RelayState;
use actix_web::{get, web, HttpResponse};

#[get("/certificate.sha256")]
pub async fn get_fingerprint(state: web::Data<RelayState>) -> HttpResponse {
    let fingerprint = state
        .tls_info
        .read()
        .expect("tls_info lock poisoned")
        .fingerprints
        .first()
        .expect("missing certificate")
        .clone();

    HttpResponse::Ok()
        .content_type("text/plain")
        .body(fingerprint)
}
