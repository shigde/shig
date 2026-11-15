use crate::db::user_roles::Role;
use crate::models::auth::session::Session;
use actix_web::{get, web, HttpResponse, Responder};

#[get("/metrics")]
pub async fn prometheus(session: web::ReqData<Session>) -> impl Responder {
    if session.principal.user_role_id != Role::Admin.val() {
        return HttpResponse::Forbidden().finish();
    }
    match crate::metrics::render() {
        Ok(body) => HttpResponse::Ok()
            .content_type("text/plain; version=0.0.4; charset=utf-8")
            .body(body),
        Err(error) => HttpResponse::InternalServerError().body(error.to_string()),
    }
}

#[cfg(test)]
#[path = "prometheus_test.rs"]
mod tests;
