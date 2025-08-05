use crate::db::DbPool;
use crate::models::auth::session::Session;
use crate::models::error::ApiError;
use crate::models::http::response::Body;
use crate::models::user::lobby::Lobby;
use actix_web::{put, web, HttpResponse};

// PUT api/user/stream/:stream_id/lobby
#[put("/{uuid}")]
pub async fn open_lobby(
    pool: web::Data<DbPool>,
    sfu_addr: web::Data<actix::Addr<crate::sfu::Sfu>>,
    path: web::Path<String>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let lobby = Lobby::open(
        &pool,
        path.into_inner(),
        session.principal.clone(),
        sfu_addr.clone(),
    )?;
    Ok(HttpResponse::Ok().json(Body::new("ok", lobby)))
}
