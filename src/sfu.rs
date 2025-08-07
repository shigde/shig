use crate::models::user::lobby::Lobby as LobbyModel;
use crate::sfu::config::SfuConfig;
use crate::sfu::error::{LobbyError, LobbyResult, SfuError, SfuResult};
use crate::sfu::lobby::{JoinPeer, Lobby, LobbyShutdown};
use actix::prelude::*;
use std::collections::HashMap;

pub mod config;
pub mod error;
pub mod lobby;
pub mod peer;
pub mod router;

pub struct Sfu {
    config: SfuConfig,
    lobbies: Box<HashMap<String, Addr<Lobby>>>,
    shutting_down: bool,
}

impl Sfu {
    pub fn new(config: SfuConfig) -> Sfu {
        let lobbies = Box::new(HashMap::new());
        Sfu {
            config,
            lobbies,
            shutting_down: false,
        }
    }
}

// Provide Actor implementation for our actor
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
    pub lobby: LobbyModel,
    pub user_uuid: String,
}

impl Handler<SartLobby> for Sfu {
    type Result = ResponseActFuture<Self, SfuResult<String>>;

    fn handle(&mut self, msg: SartLobby, ctx: &mut Self::Context) -> Self::Result {
        let lobby_uuid = msg.lobby.uuid.clone();
        if self.lobbies.contains_key(&lobby_uuid) {
            return Box::pin(fut::err(SfuError::LobbyAlreadyStarted()));
        }
        let lobby_addr = Lobby::new(msg.lobby, ctx.address()).start();
        self.lobbies.insert(lobby_uuid, lobby_addr.clone());

        let fut = async move {
            let result = lobby_addr
                .send(JoinPeer {
                    user_uuid: "".to_string(),
                    offer: "".to_string(),
                    role: peer::PeerRole::Owner,
                })
                .await;

            match result {
                Ok(val) => match val {
                    Ok(_) => Ok("".to_string()),
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
