use crate::db::DbPool;
use crate::models::auth::session::Session;
use crate::models::error::ApiError;
use crate::models::http::response::Body;
use crate::models::http::{MESSAGE_CREATED, MESSAGE_OK};
use crate::models::user::stream_friend::StreamFriend;
use actix_web::{delete, get, post, web, HttpResponse};
use serde::Deserialize;

#[allow(unused)]
#[derive(Deserialize)]
struct StreamPath {
    pub stream_uuid: String,
    pub friend_uuid: String,
}

// GET api/stream/:stream_uuid/friend/:friend_uuid
#[get("/{stream_uuid}/friend/{friend_uuid}")]
pub async fn get_stream_friend(
    pool: web::Data<DbPool>,
    path: web::Path<StreamPath>,
) -> Result<HttpResponse, ApiError> {
    let friend = StreamFriend::fetch(
        &pool,
        path.stream_uuid.to_string(),
        path.friend_uuid.to_string(),
    )?;
    Ok(HttpResponse::Ok().json(Body::new("ok", friend)))
}

// GET api/stream/:uuid/friends
#[get("/{stream_uuid}/friends")]
pub async fn get_all_stream_friends(
    pool: web::Data<DbPool>,
    stream_uuid: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let friends = StreamFriend::fetch_all(
        &pool,
        stream_uuid.into_inner(),
    )?;
    Ok(HttpResponse::Ok().json(Body::new(MESSAGE_OK, friends)))
}

// POST api/stream/:stream_uuid/friend/:friend_uuid
#[post("/{stream_uuid}/friend/{friend_uuid}")]
pub async fn create_stream_friend(
    pool: web::Data<DbPool>,
    path: web::Path<StreamPath>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let result = StreamFriend::create(
        &pool,
        path.stream_uuid.to_string(),
        path.friend_uuid.to_string(),
        session.principal.clone(),
    )?;
    Ok(HttpResponse::Created().json(Body::new(MESSAGE_CREATED, result)))
}

// DELETE api/stream/:id/friend/:friend_id
#[delete("/{stream_uuid}/friend/{friend_uuid}")]
pub async fn delete_stream_friend(
    pool: web::Data<DbPool>,
    path: web::Path<StreamPath>,
    session: web::ReqData<Session>,
) -> Result<HttpResponse, ApiError> {
    let result = StreamFriend::delete(
        &pool,
        path.stream_uuid.to_string(),
        path.friend_uuid.to_string(),
        session.principal.clone(),
    )?;
    Ok(HttpResponse::Created().json(Body::new(MESSAGE_CREATED, result)))
}
