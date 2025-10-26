use crate::db::channel_friends::read::is_channel_friend;
use crate::db::channels::read::find_channel_by_uuid;
use crate::db::lobbies::read::find_lobby_by_channel_id;
use crate::db::stream_friends::read::is_stream_friend;
use crate::db::streams::read::find_stream_by_uuid;
use crate::db::DbPool;
use crate::models::auth::session::Principal;
use crate::models::error::ApiError;
use crate::sfu::{Sfu, SubscribeLobby};
use actix::Addr;
use actix_web::web;

#[allow(dead_code)]
pub(crate) async fn whep(
    pool: &web::Data<DbPool>,
    channel_uuid: String,
    stream_uuid: String,
    user: Principal,
    sfu_addr: web::Data<Addr<Sfu>>,
    offer: String,
) -> Result<String, ApiError> {
    let mut conn = pool.get()?;
    let db_channel = find_channel_by_uuid(&mut conn, channel_uuid)?;
    let db_lobby = find_lobby_by_channel_id(&mut conn, db_channel.id)?;
    let db_stream = find_stream_by_uuid(&mut conn, stream_uuid)?;

    if db_stream.channel_id != db_channel.id {
        return Err(ApiError::Conflict {
            error_message: "stream is not in channel".to_string(),
        });
    }

    if !db_lobby.is_open {
        return Err(ApiError::Forbidden {
            error_message: "forbidden to subscribe".to_string(),
        });
    }

    let is_stream_friend = is_stream_friend(&mut conn, user.id, db_stream.id)?;
    let is_channel_friend = is_channel_friend(&mut conn, user.id, db_channel.id)?;
    let is_owner = db_lobby.user_id == user.id;

    if !is_stream_friend && !is_channel_friend && !is_owner {
        return Err(ApiError::Forbidden {
            error_message: "user is not allowed to subscribe".to_string(),
        });
    }

    let answer = match sfu_addr
        .send(SubscribeLobby {
            lobby_uuid: db_lobby.uuid.clone(),
            user_uuid: user.user_uuid.clone(),
            offer,
        })
        .await
    {
        Ok(result) => result.unwrap_or_else(|e| {
            log::error!("sfu error: {}", e);
            "answer error".to_string()
        }),
        Err(e) => {
            log::error!("sfu error: {}", e);
            return Err(ApiError::InternalServerError {
                error_message: "sfu mail error".to_string(),
            });
        }
    };

    Ok(answer)
}
