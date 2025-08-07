use crate::sfu::lobby::{Lobby, LobbyShutdown, PeerStopped};
use actix::{Actor, ActorContext, Addr, Context, Handler, Message};

pub enum PeerRole {
    Owner,
    Guest,
}

pub struct Peer {
    pub id: String,
    parent_addr: Addr<Lobby>,
}

impl Peer {
    pub fn new(id: String, parent_addr: Addr<Lobby>) -> Self {
        Self { id, parent_addr }
    }

    fn stop(&self, ctx: &mut Context<Peer>) {
        self.parent_addr.do_send(PeerStopped {
            id: self.id.clone(),
        });
        ctx.stop();
    }
}

impl Actor for Peer {
    type Context = actix::Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        log::info!("started: peer actor {} is alive", self.id);
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        log::info!("stopped: peer actor {} is stopped", self.id);
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct PeerShutdown {}

impl Handler<PeerShutdown> for Peer {
    type Result = ();

    fn handle(&mut self, _msg: PeerShutdown, ctx: &mut Self::Context) -> Self::Result {
        //@Todo implement shutdown logic
        self.stop(ctx);
    }
}
