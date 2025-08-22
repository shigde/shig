use crate::sfu::config::SfuConfig;
use crate::sfu::error::{SfuError, SfuResult};
use crate::sfu::lobby::{JoinPeer, Lobby, LobbyShutdown, SubscribeToPeers};
use actix::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use std::collections::HashMap;

pub mod config;
pub mod error;
pub mod lobby;
mod media;
mod message;
pub mod peer;

pub struct Sfu {
    _config: SfuConfig,
    lobbies: Box<HashMap<String, Addr<Lobby>>>,
    shutting_down: bool,
    _pool: Pool<ConnectionManager<PgConnection>>,
}

impl Sfu {
    pub fn new(config: SfuConfig, pool: Pool<ConnectionManager<PgConnection>>) -> Sfu {
        let lobbies = Box::new(HashMap::new());
        Sfu {
            _config: config,
            lobbies,
            shutting_down: false,
            _pool: pool,
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
pub struct SartLobby {
    pub offer: String,
    pub lobby_uuid: String,
    pub user_uuid: String,
}

impl Handler<SartLobby> for Sfu {
    type Result = ResponseActFuture<Self, SfuResult<String>>;

    fn handle(&mut self, msg: SartLobby, ctx: &mut Self::Context) -> Self::Result {
        let lobby_uuid = msg.lobby_uuid.clone();
        if self.lobbies.contains_key(&lobby_uuid) {
            return Box::pin(fut::err(SfuError::LobbyAlreadyStarted()));
        }
        let lobby_addr =
            Lobby::new(msg.lobby_uuid.clone(), msg.user_uuid.clone(), ctx.address()).start();
        self.lobbies.insert(lobby_uuid, lobby_addr.clone());

        let user_uuid = msg.user_uuid.clone();
        let offer = msg.offer;
        let fut = async move {
            let result = lobby_addr
                .send(JoinPeer {
                    user_uuid,
                    offer,
                    role: peer::PeerRole::Host,
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
pub struct JoinLobby {
    pub offer: String,
    pub lobby_uuid: String,
    pub user_uuid: String,
    pub role: peer::PeerRole,
}

impl Handler<JoinLobby> for Sfu {
    type Result = ResponseActFuture<Self, SfuResult<String>>;

    fn handle(&mut self, msg: JoinLobby, _ctx: &mut Self::Context) -> Self::Result {
        let lobby_uuid = msg.lobby_uuid.clone();

        let lobby_addr = match self.lobbies.get(&lobby_uuid) {
            None => {
                return Box::pin(fut::err(SfuError::LobbyNotExists()));
            }
            Some(lobby_addr) => lobby_addr.clone(),
        };

        let user_uuid = msg.user_uuid.clone();
        let offer = msg.offer.clone();
        let fut = async move {
            let result = lobby_addr
                .send(JoinPeer {
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
    pub offer: String,
    pub lobby_uuid: String,
    pub user_uuid: String,
}

impl Handler<SubscribeLobby> for Sfu {
    type Result = ResponseActFuture<Self, SfuResult<String>>;

    fn handle(&mut self, msg: SubscribeLobby, _ctx: &mut Self::Context) -> Self::Result {
        let lobby_uuid = msg.lobby_uuid.clone();

        let lobby_addr = match self.lobbies.get(&lobby_uuid) {
            None => {
                return Box::pin(fut::err(SfuError::LobbyNotExists()));
            }
            Some(lobby_addr) => lobby_addr.clone(),
        };

        let user_uuid = msg.user_uuid.clone();
        let offer = msg.offer.clone();
        let fut = async move {
            let result = lobby_addr.send(SubscribeToPeers { user_uuid, offer }).await;

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

        if self.shutting_down && self.lobbies.is_empty() {
            ctx.stop();
        }
    }
}
