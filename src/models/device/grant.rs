use crate::db::{device_grants, devices, lobbies, streams};
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
    session_seconds: i64,
) -> Result<DeviceSession, ApiError> {
    if !is_credential(token) {
        return Err(invalid_invite());
    }
    let invite_hash = digest(token);
    conn.transaction(|conn| {
        let invite =
            device_grants::read::find_invite(conn, &invite_hash)?.ok_or_else(invalid_invite)?;
        let device_name = devices::read::lock_active_name(conn, invite.device_uuid)?
            .ok_or_else(invalid_invite)?;
        let now = Utc::now();
        let expires_at = TimeDelta::try_seconds(session_seconds)
            .and_then(|ttl| now.checked_add_signed(ttl))
            .ok_or_else(|| ApiError::InternalServerError {
                error_message: "Invalid device session lifetime".into(),
            })?;

        let session_token = credential()?;
        let session = device_grants::update::redeem_invite(
            conn,
            invite.device_uuid,
            &invite_hash,
            &digest(&session_token),
            now,
            expires_at,
        )?
        .ok_or_else(invalid_invite)?;
        let stream_uuid = streams::read::find_stream_uuid_by_id(conn, session.stream_id)?;
        Ok(DeviceSession {
            token: session_token,
            device_name,
            stream_uuid,
        })
    })
}

pub fn end_session(conn: &mut PgConnection, token: &str) -> Result<(), ApiError> {
    if is_credential(token) {
        device_grants::delete::delete_by_session(conn, &digest(token))?;
    }
    Ok(())
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
