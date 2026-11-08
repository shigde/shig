use super::grant;
use crate::db::{device_grants, devices, lobbies, streams, DbPool};
use crate::models::auth::session::Session;
use crate::models::error::ApiError;
use crate::sfu::{peer::PeerRole, PublishLobby, Sfu};
use actix::Addr;
use diesel::Connection;

pub async fn publish(
    pool: &DbPool,
    sfu: &Addr<Sfu>,
    session: &Session,
    offer: String,
) -> Result<String, ApiError> {
    let scope = session.device_scope()?;
    let request = pool.get()?.transaction(|conn| -> Result<_, ApiError> {
        let grant = device_grants::read::lock_session(conn, &grant::session_hash(scope))?
            .filter(|grant| grant.device_uuid == scope.device_id && grant.stream_id == scope.stream_id)
            .ok_or_else(|| ApiError::Unauthorized {
                error_message: "Camera session is invalid or expired".into(),
            })?;
        let name = devices::read::find_active_name(conn, session.principal.id, scope.device_id)?
            .ok_or_else(|| ApiError::Forbidden {
                error_message: "Camera is no longer available".into(),
            })?;
        let lobby = lobbies::read::find_open_lobby_for_stream(conn, grant.stream_id)?
            .filter(|lobby| lobby.user_id == session.principal.id)
            .ok_or_else(|| ApiError::Conflict {
                error_message: "The host must be in the stream lobby".into(),
            })?;
        // Queue admission while revocation is still excluded by the grant row lock.
        Ok(sfu.send(PublishLobby {
            lobby_uuid: lobby.uuid,
            stream_uuid: streams::read::find_stream_uuid_by_id(conn, grant.stream_id)?,
            user_uuid: scope.device_id.to_string(),
            host_uuid: session.principal.user_uuid.clone(),
            offer,
            role: PeerRole::Device {
                name: name.replace(['\r', '\n'], " "),
            },
        }))
    })?;

    Ok(request.await.map_err(|_| ApiError::ServiceUnavailable {
        error_message: "SFU is unavailable".into(),
    })??)
}
