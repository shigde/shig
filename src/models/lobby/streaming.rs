use crate::db::channels::read::find_channel_by_uuid;
use crate::db::lobbies::read::find_lobby_by_channel_id;
use crate::db::streams::read::find_stream_by_uuid;
use crate::db::DbPool;
use crate::models::auth::session::Principal;
use crate::models::error::ApiError;
use crate::sfu::{PublishLobbyStream, Sfu};
use actix::Addr;
use actix_web::web;

pub(crate) async fn publish(
    pool: &web::Data<DbPool>,
    channel_uuid: String,
    stream_uuid: String,
    user: Principal,
    sfu_addr: web::Data<Addr<Sfu>>,
    publishing: bool,
) -> Result<(), ApiError> {
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

    let is_owner = db_lobby.user_id == user.id;

    if !is_owner {
        return Err(ApiError::Forbidden {
            error_message: format!(
                "user is not allowed to start streaming channel_uuid={}, stream_uuid={}, user_uuid={}",
                channel_uuid, stream_uuid, user.user_uuid
            ),
        });
    }

    if !db_lobby.is_open {
        return Err(ApiError::Forbidden {
            error_message: format!(
                "Streaming can't start, Lobby is not open, channel_uuid={}, stream_uuid={}, user_uuid={}",
                channel_uuid, stream_uuid, user.user_uuid
            ),
        });
    }

    let _ = sfu_addr
        .send(PublishLobbyStream {
            publishing,
            lobby_uuid: db_lobby.uuid.clone(),
        })
        .await
        .map_err(|e| {
            let error_message = format!(
                "internal message error on publish {} streaming, channel_uuid={}, stream_uuid={}, user_uuid={}",
                publishing, channel_uuid, stream_uuid, user.user_uuid
            );
            log::error!("{}: {}", error_message, e);
            ApiError::InternalServerError { error_message }
        })?;

    Ok(())
}
