use crate::api::relay::auth::AuthQuery;
use crate::relay::state::RelayState;
use actix_web::error::ErrorUnauthorized;
use actix_web::{get, web, HttpResponse};
use moq_relay::{AuthError, AuthParams};

#[get("/announced")]
pub async fn get_root(
    state: web::Data<RelayState>,
    query: web::Query<AuthQuery>,
) -> actix_web::Result<HttpResponse> {
    get_announced_inner(state, String::new(), query.into_inner()).await
}

#[get("/announced/{prefix:.*}")]
pub async fn get_prefix(
    state: web::Data<RelayState>,
    path: web::Path<String>,
    query: web::Query<AuthQuery>,
) -> actix_web::Result<HttpResponse> {
    get_announced_inner(state, path.into_inner(), query.into_inner()).await
}

async fn get_announced_inner(
    state: web::Data<RelayState>,
    prefix: String,
    query: AuthQuery,
) -> actix_web::Result<HttpResponse> {
    let params = AuthParams {
        path: prefix,
        jwt: query.jwt,
    };

    let token = state
        .auth
        .verify(&params)
        .await
        .map_err(|_: AuthError| ErrorUnauthorized("invalid token"))?;

    let Some(mut origin) = state.cluster.subscriber(&token) else {
        return Err(ErrorUnauthorized("unauthorized"));
    };

    let mut broadcasts = Vec::new();

    while let Some((suffix, active)) = origin.try_announced() {
        if active.is_some() {
            broadcasts.push(suffix.to_string());
        }
    }

    Ok(HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .body(broadcasts.join("\n")))
}