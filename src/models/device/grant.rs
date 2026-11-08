use crate::db::{device_grants, devices, lobbies, streams, users};
use crate::models::auth::{jwt::{JWTConfig, JWT}, scopes::DeviceScope};
use crate::models::error::ApiError;
use chrono::{TimeDelta, Utc};
use diesel::{Connection, PgConnection};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSession {
    pub token: String,
    pub device_name: String,
    pub stream_uuid: String,
}

pub fn issue_invite(
    conn: &mut PgConnection,
    owner: i32,
    uuid: Uuid,
    stream_uuid: &str,
) -> Result<String, ApiError> {
    conn.transaction(|conn| {
        let stream_id = streams::read::lock_owned_stream_id(conn, stream_uuid, owner)?;
        if lobbies::read::lock_open_lobby_for_stream(conn, stream_id, owner)?.is_none() {
            return Err(ApiError::Conflict {
                error_message: "Join the stream lobby before inviting a device".into(),
            });
        }
        let device = devices::read::lock_owned(conn, owner, uuid)?;
        if !device.active {
            return Err(ApiError::Conflict {
                error_message: "Enable the device before inviting it".into(),
            });
        }
        let now = Utc::now();
        if device_grants::read::has_active_session(conn, uuid, now)? {
            return Err(ApiError::Conflict {
                error_message: "This device already has an active session".into(),
            });
        }

        let token = credential()?;
        device_grants::create::store_invite(
            conn,
            uuid,
            stream_id,
            &digest(&token),
            now + TimeDelta::minutes(5),
        )?;
        Ok(token)
    })
}

pub fn redeem(
    conn: &mut PgConnection,
    token: &str,
    config: &JWTConfig,
) -> Result<DeviceSession, ApiError> {
    if !is_credential(token) {
        return Err(invalid_invite());
    }
    let invite_hash = digest(token);
    conn.transaction(|conn| {
        let invite =
            device_grants::read::find_invite(conn, &invite_hash)?.ok_or_else(invalid_invite)?;
        let device = devices::read::lock_active(conn, invite.device_uuid)?
            .ok_or_else(invalid_invite)?;
        let now = Utc::now();
        let expires_at = TimeDelta::try_seconds(config.session_live_time)
            .and_then(|ttl| now.checked_add_signed(ttl))
            .ok_or_else(|| ApiError::InternalServerError {
                error_message: "Invalid device session lifetime".into(),
            })?;

        let scope = DeviceScope {
            device_id: device.uuid,
            stream_id: invite.stream_id,
            session_id: Uuid::new_v4(),
        };
        let owner = users::read::find_user_by_id(conn, device.owner_user_id)?;
        if !owner.active {
            return Err(invalid_invite());
        }
        let session_token = JWT::generate_device_token(
            &owner, scope.clone(), now.timestamp(), expires_at.timestamp(), config,
        )?;
        let session = device_grants::update::redeem_invite(
            conn,
            invite.device_uuid,
            &invite_hash,
            &session_hash(&scope),
            now,
            expires_at,
        )?
        .ok_or_else(invalid_invite)?;
        let stream_uuid = streams::read::find_stream_uuid_by_id(conn, session.stream_id)?;
        Ok(DeviceSession {
            token: session_token,
            device_name: device.name,
            stream_uuid,
        })
    })
}

pub fn end_session(conn: &mut PgConnection, scope: &DeviceScope) -> Result<Option<String>, ApiError> {
    let lobby = lobbies::read::find_open_lobby_for_stream(conn, scope.stream_id)?;
    let revoked = device_grants::delete::delete_by_session(conn, &session_hash(scope))?;
    Ok(lobby.filter(|_| revoked).map(|lobby| lobby.uuid))
}

pub fn is_active_session(conn: &mut PgConnection, scope: &DeviceScope, owner_id: i32) -> Result<bool, ApiError> {
    Ok(device_grants::read::is_active_for(
        conn, &session_hash(scope), scope.device_id, scope.stream_id, owner_id,
    )?)
}

pub(super) fn session_hash(scope: &DeviceScope) -> String {
    digest(&scope.session_id.to_string())
}

fn credential() -> Result<String, ApiError> {
    let mut bytes = [0u8; 32];
    openssl::rand::rand_bytes(&mut bytes).map_err(|_| ApiError::InternalServerError {
        error_message: "Could not generate device credential".into(),
    })?;
    Ok(hex::encode(bytes))
}

fn digest(token: &str) -> String {
    hex::encode(openssl::sha::sha256(token.as_bytes()))
}

fn is_credential(token: &str) -> bool {
    token.len() == 64 && token.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn invalid_invite() -> ApiError {
    ApiError::Unauthorized {
        error_message: "Camera invitation is invalid or expired".into(),
    }
}
