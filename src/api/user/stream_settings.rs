use crate::{
    db::{user_stream_settings, DbPool},
    models::{auth::session::Session, error::ApiError, http::response::Body, user::stream_settings::{StreamSettings, StreamSettingsPatch}},
};
use actix_web::{get, patch, web, HttpResponse};

#[get("/stream-settings")]
pub async fn get(
    pool: web::Data<DbPool>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let mut conn = pool.get()?;
    let settings = user_stream_settings::read::get(&mut conn, session.principal.id)?;
    let settings = StreamSettings::from_json(settings).map_err(|error| {
        log::error!("Could not decode user stream settings: {error}");
        ApiError::InternalServerError { error_message: "Could not load stream settings".into() }
    })?;
    Ok(HttpResponse::Ok().insert_header(("Cache-Control", "no-store")).json(Body::new("ok", settings)))
}

#[patch("/stream-settings")]
pub async fn update(
    pool: web::Data<DbPool>,
    session: web::ReqData<Session>,
    patch: web::Json<StreamSettingsPatch>,
) -> Result<HttpResponse, ApiError> {
    if !patch.validate() {
        return Err(ApiError::BadRequest { error_message: "Invalid stream settings".into() });
    }
    let settings = serde_json::to_string(&patch.into_inner()).map_err(|error| {
        log::error!("Could not encode user stream settings: {error}");
        ApiError::InternalServerError { error_message: "Could not save stream settings".into() }
    })?;
    let mut conn = pool.get()?;
    user_stream_settings::set(&mut conn, session.principal.id, &settings)?;
    let saved = StreamSettings::from_json(user_stream_settings::read::get(&mut conn, session.principal.id)?)
        .map_err(|_| ApiError::InternalServerError { error_message: "Could not load stream settings".into() })?;
    Ok(HttpResponse::Ok().insert_header(("Cache-Control", "no-store")).json(Body::new("ok", saved)))
}
