use crate::db::error::DbResult;
use crate::db::lobbies::read::find_lobby_by_uuid;
use crate::db::lobbies::update::update_lobby;
use crate::db::streams::read::find_stream_by_uuid;
use crate::db::DbPool;
use crate::models::auth::session::Principal;
use crate::models::error::ApiError;
use crate::sfu::{SartLobby, Sfu};
use actix::Addr;
use actix_web::web;
use diesel::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Lobby {
    pub uuid: String,
    // user name@domain
    pub user: String,
    pub name: String,
    pub description: String,
    pub support: String,
    pub is_open: bool,
    pub banner_name: String,
}

impl Lobby {
    pub fn new() -> Self {
        Self {
            uuid: String::new(),
            user: String::new(),
            name: String::new(),
            description: String::new(),
            support: String::new(),
            is_open: false,
            banner_name: String::new(),
        }
    }

    pub(crate) fn open(
        pool: &web::Data<DbPool>,
        lobby_id: String,
        stream_id: String,
        secret: String,
        user: Principal,
        sfu_addr: web::Data<Addr<Sfu>>,
    ) -> Result<Self, ApiError> {
        let mut conn = pool.get()?;
        let lobby = conn.transaction(move |conn| -> Result<Lobby, ApiError> {
            let db_lobby = find_lobby_by_uuid(conn, lobby_id)?;

            if db_lobby.user_id != user.id || db_lobby.secret != secret {
                return Err(ApiError::Unauthorized {
                    error_message: "request not allowed".to_string(),
                });
            }

            if db_lobby.is_open {
                return Err(ApiError::Conflict {
                    error_message: "lobby is already open".to_string(),
                });
            }

            let full_stream = find_stream_by_uuid(conn, stream_id)?;
            let stream = full_stream.stream;

            update_lobby(
                conn,
                db_lobby.uuid.as_str(),
                Some(stream.id),
                db_lobby.secret.as_str(),
                true,
            )?;

            let mut lobby = Lobby::new();
            let _= sfu_addr.try_send(SartLobby{
                lobby: lobby.clone(),
                user_uuid: user.user_uuid.clone(),
            });
            
            // lobby.is_open = true;
            Ok(lobby)
        });

        lobby
    }
}
