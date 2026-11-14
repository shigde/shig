use crate::{
    db::{streams::read::find_stream_by_uuid, DbPool},
    ephemeral::state::EphemeralState,
    models::{auth::session::Session, error::ApiError, http::response::Body},
    relay::state::RelayState,
};
use actix_web::{post, web, HttpRequest, HttpResponse};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewerHeartbeat {
    #[serde(rename = "viewerId")]
    viewer_id: String,
}

#[post("/{uuid}")]
pub async fn poll(
    pool: web::Data<DbPool>,
    state: web::Data<RelayState>,
    ephemeral: web::Data<EphemeralState>,
    request: HttpRequest,
    path: web::Path<String>,
    session: Option<web::ReqData<Session>>,
    body: web::Json<ViewerHeartbeat>,
) -> Result<HttpResponse, ApiError> {
    let uuid = path.into_inner();
    uuid::Uuid::parse_str(&uuid).map_err(|_| ApiError::BadRequest {
        error_message: "Invalid stream UUID".into(),
    })?;
    let token = uuid::Uuid::parse_str(&body.viewer_id).map_err(|_| ApiError::BadRequest {
        error_message: "Invalid viewer token".into(),
    })?;
    let (stream, channel_uuid) = {
        let mut conn = pool.get()?;
        let stream = find_stream_by_uuid(&mut conn, uuid.clone())?;
        let channel = crate::db::channels::read::find_channel_by_id(&mut conn, stream.channel_id)?;
        (stream, channel.uuid)
    };
    if !stream.is_public && session.is_none() {
        return Err(ApiError::NotFound {
            error_message: "Stream is not available".into(),
        });
    }
    if !stream.is_public {
        let Some(session) = session.as_ref() else {
            return Err(ApiError::NotFound {
                error_message: "Stream is not available".into(),
            });
        };
        crate::models::user::stream::Stream::fetch(&pool, uuid.clone(), session.principal.clone())?;
    }
    let announcements = crate::api::relay::announcement::announced_prefix(
        &state,
        &request,
        &format!("live/{channel_uuid}"),
    )
    .await
    .map_err(|_| ApiError::ServiceUnavailable {
        error_message: "Stream status is unavailable".into(),
    })?;
    let is_live = announcements.lines().any(|name| name.trim() == uuid);
    let viewers = if is_live {
        let key = session.as_ref()
            .map(|s| format!("account:{}", s.principal.id))
            .unwrap_or_else(|| format!("anonymous:{token}"));
        ephemeral.viewer.heartbeat(&uuid, &key)
    } else {
        ephemeral.viewer.clear_stream(&uuid);
        0
    };
    Ok(HttpResponse::Ok().json(Body::new(
        "ok",
        serde_json::json!({"viewers": viewers, "isLive": is_live}),
    )))
}
