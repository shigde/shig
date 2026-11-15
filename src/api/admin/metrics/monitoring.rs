use crate::db::user_roles::Role;
use crate::models::auth::session::Session;
use actix_web::{get, web, HttpResponse};

#[get("/monitoring")]
pub async fn monitoring(session: web::ReqData<Session>) -> HttpResponse {
    if session.principal.user_role_id != Role::Admin.val() {
        return HttpResponse::Forbidden().finish();
    }
    HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-store"))
        .json(crate::metrics::snapshot())
}

#[cfg(test)]
#[path = "monitoring_test.rs"]
mod tests;
