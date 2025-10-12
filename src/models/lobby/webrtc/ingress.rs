use crate::db::channel_friends::read::is_channel_friend;
use crate::db::channels::read::find_channel_by_uuid;
use crate::db::lobbies::read::find_lobby_by_channel_id;
use crate::db::lobbies::update::update_lobby;
use crate::db::stream_friends::read::is_stream_friend;
use crate::db::streams::read::find_stream_by_uuid;
use crate::db::DbPool;
use crate::models::auth::session::Principal;
use crate::models::error::ApiError;
use crate::sfu::peer::PeerRole;
use crate::sfu::{JoinLobby, Sfu};
use actix::Addr;
use actix_web::web;

#[allow(dead_code)]
pub(crate) async fn whip(
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

    let is_stream_friend = is_stream_friend(&mut conn, user.id, db_stream.id)?;
    let is_channel_friend = is_channel_friend(&mut conn, user.id, db_channel.id)?;
    let is_owner = db_lobby.user_id == user.id;

    if !is_stream_friend && !is_channel_friend && !is_owner {
        return Err(ApiError::Unauthorized {
            error_message: "user is not authorized to join".to_string(),
        });
    }

    let role = if is_owner {
        PeerRole::Host
    } else {
        PeerRole::Guest
    };
    let answer = match sfu_addr
        .send(JoinLobby {
            lobby_uuid: db_lobby.uuid.clone(),
            user_uuid: user.user_uuid.clone(),
            offer,
            role,
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
    // update_lobby(&mut conn, db_lobby.uuid.as_str(), Some(db_stream.id), true)?;
    Ok(answer)
}
