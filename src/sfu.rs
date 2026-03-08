use crate::sfu::config::SfuConfig;
use crate::sfu::db::message::{SetLobbyOffline, SetLobbyOnline};
use crate::sfu::db::DbActor;
use crate::sfu::error::{SfuError, SfuResult};
use crate::sfu::lobby::{Lobby, LobbyShutdown, Publish, Subscribe, SubscribeKind};
use actix::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use std::collections::HashMap;

pub mod config;
pub mod db;
pub mod error;
pub mod lobby;
mod media;
mod message;
pub mod peer;

pub struct Sfu {
    _config: SfuConfig,
    lobbies: Box<HashMap<String, Addr<Lobby>>>,
    shutting_down: bool,
    db_actor: Addr<DbActor>,
}

impl Sfu {
    pub fn new(config: SfuConfig, pool: Pool<ConnectionManager<PgConnection>>) -> Sfu {
        let lobbies = Box::new(HashMap::new());
        let db_actor = SyncArbiter::start(1, move || DbActor::new(pool.clone()));
        Sfu {
            _config: config,
            lobbies,
            shutting_down: false,
            db_actor,
        }
    }
}

// Provide Actor implementation for our SFU
impl Actor for Sfu {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        log::info!("Sfu actor is alive");
    }

    fn stopping(&mut self, _ctx: &mut Context<Self>) -> Running {
        log::info!("Sfu actor is stopping");
        Running::Stop
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        log::info!("Sfu actor is stopped");
    }
}

#[derive(Message)]
#[rtype(result = " SfuResult<String>")]
pub struct PublishLobby {
    pub offer: String,
    pub lobby_uuid: String,
    pub stream_uuid: String,
    pub user_uuid: String,
    pub role: peer::PeerRole,
}

impl Handler<PublishLobby> for Sfu {
    type Result = ResponseActFuture<Self, SfuResult<String>>;

    fn handle(&mut self, msg: PublishLobby, ctx: &mut Self::Context) -> Self::Result {
        let lobby_uuid = msg.lobby_uuid.clone();

        let lobby_addr = match self.lobbies.get(&lobby_uuid) {
            None => {
                let lobby_addr = Lobby::new(
                    msg.lobby_uuid.clone(),
                    msg.stream_uuid.clone(),
                    msg.user_uuid.clone(),
                    ctx.address(),
                    self.db_actor.clone(),
                )
                .start();
                self.lobbies.insert(lobby_uuid.clone(), lobby_addr.clone());
                self.db_actor.do_send(SetLobbyOnline {
                    lobby_uuid: lobby_uuid.clone(),
                });
                lobby_addr.clone()
            }
            Some(lobby_addr) => lobby_addr.clone(),
        };

        let user_uuid = msg.user_uuid.clone();
        let offer = msg.offer.clone();

        let fut = async move {
            log::info!(
                "peer joining lobby,  peer_id={}, lobby_id={}",
                user_uuid.clone(),
                lobby_uuid.clone()
            );
            let result = lobby_addr
                .send(Publish {
                    user_uuid,
                    offer,
                    role: msg.role,
                })
                .await;

            match result {
                Ok(val) => match val {
                    Ok(answer) => Ok(answer),
                    Err(e) => Err(SfuError::LobbyError(e)),
                },
                Err(e) => Err(SfuError::LobbyMailboxError(e)),
            }
        }
        .into_actor(self);

        Box::pin(fut)
    }
}

#[derive(Message)]
#[rtype(result = " SfuResult<String>")]
pub struct SubscribeLobby {
    pub kind: SubscribeKind,
    pub answer: Option<String>,
    pub lobby_uuid: String,
    pub user_uuid: String,
}

impl Handler<SubscribeLobby> for Sfu {
    type Result = ResponseActFuture<Self, SfuResult<String>>;

    fn handle(&mut self, msg: SubscribeLobby, _ctx: &mut Self::Context) -> Self::Result {
        let lobby_uuid = msg.lobby_uuid.clone();
        let kind = msg.kind;

        let lobby_addr = match self.lobbies.get(&lobby_uuid) {
            None => {
                return Box::pin(fut::err(SfuError::LobbyNotExists()));
            }
            Some(lobby_addr) => lobby_addr.clone(),
        };

        let user_uuid = msg.user_uuid.clone();
        let answer = msg.answer.clone();
        let fut = async move {
            log::info!(
                "peer subscribing lobby,  peer_id={}, lobby_id={}, kind={}",
                user_uuid.clone(),
                lobby_uuid.clone(),
                kind,
            );
            let result = lobby_addr
                .send(Subscribe {
                    kind,
                    user_uuid,
                    answer,
                })
                .await;

            match result {
                Ok(val) => match val {
                    Ok(answer) => Ok(answer),
                    Err(e) => Err(SfuError::LobbyError(e)),
                },
                Err(e) => Err(SfuError::LobbyMailboxError(e)),
            }
        }
        .into_actor(self);

        Box::pin(fut)
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Shutdown {}

impl Handler<Shutdown> for Sfu {
    type Result = ();

    fn handle(&mut self, _msg: Shutdown, ctx: &mut Self::Context) -> Self::Result {
        self.shutting_down = true;

        for (_, addr) in self.lobbies.iter() {
            addr.do_send(LobbyShutdown {});
        }

        if self.lobbies.is_empty() {
            ctx.stop();
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct LobbyStopped {
    id: String,
}

impl Handler<LobbyStopped> for Sfu {
    type Result = ();

    fn handle(&mut self, msg: LobbyStopped, ctx: &mut Context<Self>) {
        self.lobbies.remove(&msg.id);
        self.db_actor
            .do_send(SetLobbyOffline { lobby_uuid: msg.id });

        if self.shutting_down && self.lobbies.is_empty() {
            ctx.stop();
        }
    }
}
