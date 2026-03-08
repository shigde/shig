use crate::db::channel_friends::read::is_channel_friend;
use crate::db::channels::read::find_channel_by_uuid;
use crate::db::lobbies::read::find_lobby_by_channel_id;
use crate::db::stream_friends::read::is_stream_friend;
use crate::db::streams::read::find_stream_by_uuid;
use crate::db::DbPool;
use crate::models::auth::session::Principal;
use crate::models::error::ApiError;
use crate::sfu::lobby::SubscribeKind;
use crate::sfu::{Sfu, SubscribeLobby};
use actix::Addr;
use actix_web::web;

pub(crate) async fn whep_offer(
    pool: &web::Data<DbPool>,
    channel_uuid: String,
    stream_uuid: String,
    user: Principal,
    sfu_addr: web::Data<Addr<Sfu>>,
) -> Result<String, ApiError> {
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

    if !db_lobby.is_open {
        return Err(ApiError::Forbidden {
            error_message: format!(
                "Lobby is not open, channel_uuid={}, stream_uuid={}, user_uuid={}",
                channel_uuid, stream_uuid, user.user_uuid
            )
        });
    }

    let is_stream_friend = is_stream_friend(&mut conn, db_stream.id, user.id)?;
    let is_channel_friend = is_channel_friend(&mut conn, db_channel.id, user.id,)?;
    let is_owner = db_lobby.user_id == user.id;

    if !is_stream_friend && !is_channel_friend && !is_owner {
        return Err(ApiError::Forbidden {
            error_message: format!(
                "user is not allowed to subscribe stream channel_uuid={}, stream_uuid={}, user_uuid={}",
                channel_uuid, stream_uuid, user.user_uuid
            ),
        });
    }

    let answer = match sfu_addr
        .send(SubscribeLobby {
            kind: SubscribeKind::Offer,
            lobby_uuid: db_lobby.uuid.clone(),
            user_uuid: user.user_uuid.clone(),
            answer: None,
        })
        .await
    {
        Ok(result) => result.unwrap_or_else(|e| {
            let error_message = format!(
                "SFU error on offer, channel_uuid= {}, stream_uuid={}, user_uuid={}",
                channel_uuid, stream_uuid, user.user_uuid
            );
            log::error!("{}: {}", error_message.as_str(), e);
            error_message
        }),
        Err(e) => {
            let error_message = format!(
                "internal message error on join lobby, channel_uuid= {}, stream_uuid={}, user_uuid={}",
                channel_uuid, stream_uuid, user.user_uuid
            );
            log::error!("{}: {}", error_message.as_str(), e);
            return Err(ApiError::InternalServerError { error_message });
        }
    };

    Ok(answer)
}

pub(crate) async fn whep_answer(
    pool: &web::Data<DbPool>,
    channel_uuid: String,
    stream_uuid: String,
    user: Principal,
    sfu_addr: web::Data<Addr<Sfu>>,
    answer: String,
) -> Result<String, ApiError> {
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

    if !db_lobby.is_open {
        return Err(ApiError::Forbidden {
            error_message: format!(
                "Lobby is not open, channel_uuid={}, stream_uuid={}, user_uuid={}",
                channel_uuid, stream_uuid, user.user_uuid
            ),
        });
    }

    let is_stream_friend = is_stream_friend(&mut conn, db_stream.id, user.id,)?;
    let is_channel_friend = is_channel_friend(&mut conn, db_channel.id, user.id)?;
    let is_owner = db_lobby.user_id == user.id;

    if !is_stream_friend && !is_channel_friend && !is_owner {
        return Err(ApiError::Forbidden {
            error_message: format!(
                "user is not allowed to subscribe stream channel_uuid={}, stream_uuid={}, user_uuid={}",
                channel_uuid, stream_uuid, user.user_uuid
            ),
        });
    }

    let answer = match sfu_addr
        .send(SubscribeLobby {
            kind: SubscribeKind::Answer,
            lobby_uuid: db_lobby.uuid.clone(),
            user_uuid: user.user_uuid.clone(),
            answer: Some(answer),
        })
        .await
    {
        Ok(result) => result.unwrap_or_else(|e| {
            let error_message = format!(
                "SFU error on answer, channel_uuid= {}, stream_uuid={}, user_uuid={}",
                channel_uuid, stream_uuid, user.user_uuid
            );
            log::error!("{}: {}", error_message.as_str(), e);
            error_message
        }),
        Err(e) => {
            let error_message = format!(
                "internal message error on join lobby, channel_uuid= {}, stream_uuid={}, user_uuid={}",
                channel_uuid, stream_uuid, user.user_uuid
            );
            log::error!("{}: {}", error_message.as_str(), e);
            return Err(ApiError::InternalServerError { error_message });
        }
    };

    Ok(answer)
}
