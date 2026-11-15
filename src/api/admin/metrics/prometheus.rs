use crate::db::user_roles::Role;
use crate::models::auth::session::Session;
use crate::relay::state::RelayState;
use actix_web::{get, web, HttpResponse, Responder};

#[get("/metrics")]
pub async fn prometheus(
    session: web::ReqData<Session>,
    relay: Option<web::Data<RelayState>>,
) -> impl Responder {
    if session.principal.user_role_id != Role::Admin.val() {
        return HttpResponse::Forbidden().finish();
    }
    match crate::metrics::render(relay.as_deref().map(|state| &**state)).await {
        Ok(body) => HttpResponse::Ok()
            .content_type("text/plain; version=0.0.4; charset=utf-8")
            .body(body),
        Err(error) => HttpResponse::InternalServerError().body(error.to_string()),
    }
}

#[cfg(test)]
#[path = "prometheus_test.rs"]
mod tests;
