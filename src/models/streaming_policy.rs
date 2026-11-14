use crate::db::{streaming_policy as dao, DbPool};
use crate::db::user_roles::Role;
use crate::models::auth::session::Principal;
use crate::models::error::ApiError;
use actix::Addr;
use actix_web::web;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mode { Request, Open, Locked }

impl Mode {
    pub fn as_str(self) -> &'static str { match self { Self::Request => "request", Self::Open => "open", Self::Locked => "locked" } }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyInfo { pub mode: Mode, pub channel_status: String, pub can_stream: bool }

pub fn can_stream(conn: &mut diesel::PgConnection, principal: &Principal) -> Result<bool, ApiError> {
    if principal.user_role_id == Role::Admin.val() { return Ok(true); }
    let mode = dao::mode(conn).map_err(crate::db::error::DbError::from)?;
    let status = dao::channel_status(conn, principal.channel_id).map_err(crate::db::error::DbError::from)?;
    Ok(match (mode.as_str(), status.as_deref()) {
        ("locked", _) | (_, Some("blocked")) => false,
        (_, Some("allowed")) => true,
        ("open", _) => true,
        _ => false,
    })
}

pub fn info(pool: &web::Data<DbPool>, principal: &Principal) -> Result<PolicyInfo, ApiError> {
    let mut conn = pool.get()?;
    info_for(&mut conn, principal)
}

pub fn info_for(conn: &mut diesel::PgConnection, principal: &Principal) -> Result<PolicyInfo, ApiError> {
    let mode = dao::mode(conn).map_err(crate::db::error::DbError::from)?;
    let mode = match mode.as_str() { "open" => Mode::Open, "locked" => Mode::Locked, _ => Mode::Request };
    if principal.user_role_id == Role::Admin.val() {
        return Ok(PolicyInfo { mode, channel_status: "admin".into(), can_stream: true });
    }
    let status = dao::channel_status(conn, principal.channel_id).map_err(crate::db::error::DbError::from)?
        .unwrap_or_else(|| "inherited".into());
    let can_stream = can_stream(conn, principal)?;
    Ok(PolicyInfo { mode, channel_status: status, can_stream })
}

pub fn request(conn: &mut diesel::PgConnection, channel_id: i32) -> Result<bool, ApiError> {
    if dao::mode(conn).map_err(crate::db::error::DbError::from)? != "request" {
        return Err(ApiError::Conflict { error_message: "Streaming requests are available only when the server is set to request approval".into() });
    }
    Ok(dao::request(conn, channel_id).map_err(crate::db::error::DbError::from)?)
}

pub async fn set_global(pool: &web::Data<DbPool>, mode: Mode, sfu: &Addr<crate::sfu::Sfu>) -> Result<(), ApiError> {
    let mut conn = pool.get()?;
    let active = dao::set_global(&mut conn, mode.as_str()).map_err(crate::db::error::DbError::from)?;
    for lobby_uuid in active {
        // Lobbies absent from the in-memory SFU are already stopped.
        let _ = sfu.send(crate::sfu::PublishLobbyStream { lobby_uuid, publishing: false, auth_token: None }).await;
    }
    Ok(())
}

pub async fn set_channel(pool: &web::Data<DbPool>, uuid: &str, status: &str, sfu: &Addr<crate::sfu::Sfu>) -> Result<bool, ApiError> {
    let mut conn = pool.get()?;
    let Some(change) = dao::set_channel(&mut conn, uuid, status).map_err(crate::db::error::DbError::from)? else { return Ok(false); };
    let notification = match (status, change.old_status.as_deref()) {
        ("allowed", Some("allowed")) | ("blocked", Some("blocked")) | ("inherited", None) => None,
        ("allowed", _) => Some(("Streaming permission granted", "The administrator granted your channel permission to stream.")),
        ("blocked", _) => Some(("Channel blocked from streaming", "The administrator blocked this channel from streaming and unpublished its public content.")),
        ("inherited", Some("requested")) => Some(("Streaming permission request denied", "The administrator denied this request. Your channel is back under the server default and may request permission again.")),
        ("inherited", Some("allowed")) => Some(("Streaming permission removed", "The administrator removed your individual streaming permission. The server default now applies.")),
        ("inherited", Some("blocked")) => Some(("Channel unblocked", "The administrator removed the channel block. The server default now applies.")),
        _ => None,
    };
    if let Some((title, body)) = notification {
        crate::models::inbox::streaming_permission_changed(&mut conn, change.user_id, title, body, uuid)
            .map_err(crate::db::error::DbError::from)?;
    }
    for lobby_uuid in change.lobby_uuids {
        let _ = sfu.send(crate::sfu::PublishLobbyStream { lobby_uuid, publishing: false, auth_token: None }).await;
    }
    Ok(true)
}
