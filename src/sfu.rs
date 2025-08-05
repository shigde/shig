use crate::sfu::config::SfuConfig;
use crate::sfu::lobby::Lobby;
use actix::prelude::*;
use std::collections::HashMap;

pub mod config;
pub mod error;
pub mod lobby;
pub mod peer;
pub mod router;

pub struct Sfu {
    config: SfuConfig,
    lobbies: Box<HashMap<String, Lobby>>,
}

impl Sfu {
    pub fn new(config: SfuConfig) -> Sfu {
        let lobbies = Box::new(HashMap::new());
        Sfu { config, lobbies }
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
#[rtype(result = "()")]
pub struct SartLobby {
    lobby_id: String,
    owner_id: String,
    stream_id: String,
}

impl Handler<SartLobby> for Sfu {
    type Result = ();

    fn handle(&mut self, _msg: SartLobby, _: &mut Self::Context) -> Self::Result {
        //let addr = ChildActor::new(msg.id.clone()).start();
        //self.children.insert(msg.id, addr);
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct StopNow {
}

impl Handler<StopNow> for Sfu {
    type Result = ();

    fn handle(&mut self, _msg: StopNow, _: &mut Self::Context) -> Self::Result {
        //let addr = ChildActor::new(msg.id.clone()).start();
        //self.children.insert(msg.id, addr);
    }
}