use crate::db::channel_friends::read::is_channel_friend;
use crate::db::channels::read::find_channel_by_uuid;
use crate::db::lobbies::read::find_lobby_by_channel_id;
use crate::db::stream_friends::read::is_stream_friend;
use crate::db::stream_participants::read::{
    read_stream_participant_as_active_users_by_stream_uuid,
    read_stream_participant_by_user_and_stream_uuid,
};
use crate::db::streams::read::find_stream_by_uuid;
use crate::db::DbPool;
use crate::models::auth::session::Principal;
use crate::models::error::ApiError;
use crate::models::user::stream_participant::StreamParticipant;
use crate::sfu::{LeaveLobby, Sfu};
use actix::Addr;
use actix_web::web;

pub mod macros;
pub mod streaming;
pub mod webrtc;

pub(crate) async fn leave_lobby(
    pool: &web::Data<DbPool>,
    channel_uuid: String,
    stream_uuid: String,
    user: Principal,
    sfu_addr: web::Data<Addr<Sfu>>,
) -> Result<(), ApiError> {
    let mut conn = pool.get()?;
    let db_channel = find_channel_by_uuid(&mut conn, channel_uuid.clone())?;
    let db_lobby = find_lobby_by_channel_id(&mut conn, db_channel.id)?;

    let Some(_participant) = read_stream_participant_by_user_and_stream_uuid(
        &mut conn,
        stream_uuid.as_str(),
        user.user_uuid.as_str(),
    )?
    else {
        return Err(ApiError::NotFound {
            error_message: format!(
                "stream participant not found, channel_uuid={}, stream_uuid={}, user_uuid={}",
                channel_uuid, stream_uuid, user.user_uuid
            ),
        });
    };

    if !db_lobby.is_open {
        return Err(ApiError::Forbidden {
            error_message: format!(
                "Lobby is not open, channel_uuid={}, stream_uuid={}, user_uuid={}",
                channel_uuid, stream_uuid, user.user_uuid
            ),
        });
    }

    let _ = sfu_addr
        .send(LeaveLobby {
            lobby_uuid: db_lobby.uuid.clone(),
            user_uuid: user.user_uuid.clone(),
        })
        .await
        .map_err(|e| {
            let error_message = format!(
                "internal message error on leave lobby, channel_uuid={}, stream_uuid={}, user_uuid={}",
                channel_uuid, stream_uuid, user.user_uuid
            );
            log::error!("{}: {}", error_message, e);
            ApiError::InternalServerError { error_message }
        })?;

    Ok(())
}

pub(crate) async fn fetch_all_participants(
    pool: &web::Data<DbPool>,
    channel_uuid: String,
    stream_uuid: String,
    user: Principal,
) -> Result<Vec<StreamParticipant>, ApiError> {
    let mut conn = pool.get()?;
    let db_channel = find_channel_by_uuid(&mut conn, channel_uuid.clone())?;
    let db_lobby = find_lobby_by_channel_id(&mut conn, db_channel.id)?;
    let db_stream = find_stream_by_uuid(&mut conn, stream_uuid.clone())?;

    if db_stream.channel_id != db_channel.id {
        return Err(ApiError::Conflict {
            error_message: format!(
                "stream is not part of this channel channel_uuid={}, stream_uuid={}, user_uuid={}",
                channel_uuid, stream_uuid, user.user_uuid
            ),
        });
    }

    let is_stream_friend = is_stream_friend(&mut conn, db_stream.id, user.id)?;
    let is_channel_friend = is_channel_friend(&mut conn, db_channel.id, user.id)?;
    let is_owner = db_lobby.user_id == user.id;

    if !is_stream_friend && !is_channel_friend && !is_owner {
        return Err(ApiError::Forbidden {
            error_message: format!(
                "user is not allowed to see lobby online state channel_uuid={}, stream_uuid={}, user_uuid={}",
                channel_uuid, stream_uuid, user.user_uuid
            ),
        });
    }

    let active_users =
        read_stream_participant_as_active_users_by_stream_uuid(&mut conn, stream_uuid.as_str())?;

    let mut participants = Vec::with_capacity(active_users.len());

    for user in active_users {
        let friend = StreamParticipant::from_active_user(user);
        participants.push(friend);
    }

    Ok(participants)
}

pub(crate) async fn is_lobby_online(
    pool: &web::Data<DbPool>,
    channel_uuid: String,
    stream_uuid: String,
    user: Principal,
) -> Result<bool, ApiError> {
    let mut conn = pool.get()?;
    let db_channel = find_channel_by_uuid(&mut conn, channel_uuid.clone())?;
    let db_lobby = find_lobby_by_channel_id(&mut conn, db_channel.id)?;
    let db_stream = find_stream_by_uuid(&mut conn, stream_uuid.clone())?;

    if db_stream.channel_id != db_channel.id {
        return Err(ApiError::Conflict {
            error_message: format!(
                "stream is not part of this channel channel_uuid={}, stream_uuid={}, user_uuid={}",
                channel_uuid, stream_uuid, user.user_uuid
            ),
        });
    }

    let is_stream_friend = is_stream_friend(&mut conn, db_stream.id, user.id)?;
    let is_channel_friend = is_channel_friend(&mut conn, db_channel.id, user.id)?;
    let is_owner = db_lobby.user_id == user.id;

    if !is_stream_friend && !is_channel_friend && !is_owner {
        return Err(ApiError::Forbidden {
            error_message: format!(
                "user is not allowed to see lobby online state channel_uuid={}, stream_uuid={}, user_uuid={}",
                channel_uuid, stream_uuid, user.user_uuid
            ),
        });
    }

    Ok(db_lobby.is_open)
}
