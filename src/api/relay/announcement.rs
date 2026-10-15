use crate::relay::state::RelayState;
use actix_web::{error::ErrorUnauthorized, get, web, HttpRequest, HttpResponse};

#[get("/announced")]
pub async fn get_root(
    state: web::Data<RelayState>,
    request: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    announced(&state, &request).await
}

#[get("/announced/{prefix:.*}")]
pub async fn get_prefix(
    state: web::Data<RelayState>,
    request: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    announced(&state, &request).await
}

async fn announced(state: &RelayState, request: &HttpRequest) -> actix_web::Result<HttpResponse> {
    let prefix = request.match_info().get("prefix").unwrap_or("");
    let mut admission = state.auth.request(moq_auth::Transport::Http, prefix);
    admission.remote = request.peer_addr();
    if !request.query_string().is_empty() {
        admission.query = Some(request.query_string().to_string());
    }
    let lease = state
        .auth
        .admit(admission)
        .await
        .map_err(|_| ErrorUnauthorized("invalid token"))?;
    let Some(origin) = state.cluster.subscriber(lease.token()) else {
        lease.close("unauthorized", Default::default());
        return Err(ErrorUnauthorized("unauthorized"));
    };
    let mut announcements = origin.consume().announced();
    let mut broadcasts = Vec::new();
    while let Some(update) = announcements.try_next() {
        if update.kind.is_active() {
            broadcasts.push(update.prefix.to_string());
        }
    }
    lease.close("done", Default::default());
    Ok(HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .body(broadcasts.join("\n")))
}
