use crate::models::user::lobby::Lobby as Model;
use crate::sfu::error::LobbyResult;
use crate::sfu::peer::{Peer, PeerRole, PeerShutdown};
use crate::sfu::router::Router;
use actix::{Actor, ActorContext, Addr, Context, Handler, Message};
use std::collections::HashMap;
use crate::sfu::{LobbyStopped, Sfu};

pub struct Lobby {
    id: String,
    model: Model,
    peers: HashMap<String, Addr<Peer>>,
    parent_addr: Addr<Sfu>,
    router: Router,
    shutting_down: bool,
}

impl Lobby {
    pub fn new(model: Model, parent_addr: Addr<Sfu>,) -> Self {
        Self {
            id: model.uuid.clone(),
            model,
            peers: HashMap::new(),
            parent_addr,
            router: Router::new(),
            shutting_down: false
        }
    }

    fn stop(&mut self, ctx: &mut Context<Self>) {
        self.parent_addr.do_send(LobbyStopped{
            id: self.id.clone()
        });
        ctx.stop();
    }
}

impl Actor for Lobby {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        log::info!("started: lobby actor {} is alive", self.id);
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        log::info!("stopped: lobby actor {} is stopped", self.id);
    }
}


#[derive(Message)]
#[rtype(result = " LobbyResult<String>")]
pub struct JoinPeer {
    pub user_uuid: String,
    pub offer: String,
    pub role: PeerRole,
}

impl Handler<JoinPeer> for Lobby {
    type Result = LobbyResult<String>;

    fn handle(&mut self, _msg: JoinPeer, _: &mut Self::Context) -> Self::Result {
        Ok("".to_string())
    }
}

#[derive(Message)]
#[rtype(result = " LobbyResult<()>")]
pub struct LeavePeer {
    user_uuid: String,
    offer: String,
    role: PeerRole,
}

impl Handler<LeavePeer> for Lobby {
    type Result = LobbyResult<()>;

    fn handle(&mut self, _msg: LeavePeer, _: &mut Self::Context) -> Self::Result {
        // send leave to peer
        Ok(())
    }
}

#[derive(Message)]
#[rtype(result = " LobbyResult<()>")]
pub struct TimeoutPeer {
    user_uuid: String,
}

impl Handler<TimeoutPeer> for Lobby {
    type Result = LobbyResult<()>;

    fn handle(&mut self, _msg: TimeoutPeer, _: &mut Self::Context) -> Self::Result {
        Ok(())
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct LobbyShutdown {}

impl Handler<LobbyShutdown> for Lobby {
    type Result = ();

    fn handle(&mut self, _msg: LobbyShutdown, ctx: &mut Self::Context) -> Self::Result {
        self.shutting_down = true;

        for (_, addr) in self.peers.iter() {
            addr.do_send(PeerShutdown{});
        }

        if self.peers.is_empty() {
            self.stop(ctx);
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct PeerStopped {
    pub id: String,
}

impl Handler<PeerStopped> for Lobby {
    type Result = ();

    fn handle(&mut self, msg: PeerStopped, ctx: &mut Self::Context) -> Self::Result {
        self.peers.remove(&msg.id);

        if self.peers.is_empty() {
            self.stop(ctx);       
        }
    }
}
