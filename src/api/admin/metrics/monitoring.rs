use crate::db::user_roles::Role;
use crate::models::auth::session::Session;
use crate::relay::state::RelayState;
use actix_web::{get, web, HttpResponse};

#[get("/monitoring")]
pub async fn monitoring(
    session: web::ReqData<Session>,
    relay: Option<web::Data<RelayState>>,
) -> HttpResponse {
    if session.principal.user_role_id != Role::Admin.val() {
        return HttpResponse::Forbidden().finish();
    }
    let disabled_registry = moq_net::stats::Registry::default();
    let relay_stats = relay
        .as_ref()
        .map(|state| &state.relay_stats)
        .unwrap_or(&disabled_registry);
    HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-store"))
        .json(crate::metrics::snapshot(relay_stats))
}

#[cfg(test)]
#[path = "monitoring_test.rs"]
mod tests;
