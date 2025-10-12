use crate::sfu::error::{LobbyError, LobbyResult};
use crate::sfu::media::router::Router;
use crate::sfu::peer::{Peer, PeerId, PeerRole, PeerShutdown, PeerStartReceiving};
use crate::sfu::{LobbyStopped, Sfu};
use actix::{
    Actor, ActorContext, Addr, AsyncContext, Context, Handler, Message, ResponseActFuture,
    WrapFuture,
};
use std::collections::HashMap;

pub struct Lobby {
    id: String,
    host_uuid: String,
    peers: Box<HashMap<PeerId, Addr<Peer>>>,
    parent_addr: Addr<Sfu>,
    router: Router,
    shutting_down: bool,
}

impl Lobby {
    pub fn new(uuid: String, host_uuid: String, parent_addr: Addr<Sfu>) -> Self {
        Self {
            id: uuid,
            host_uuid,
            peers: Box::new(HashMap::new()),
            parent_addr,
            router: Router::new(),
            shutting_down: false,
        }
    }

    fn stop(&mut self, ctx: &mut Context<Self>) {
        self.parent_addr.do_send(LobbyStopped {
            id: self.id.clone(),
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
    type Result = ResponseActFuture<Self, LobbyResult<String>>;

    fn handle(&mut self, msg: JoinPeer, ctx: &mut Self::Context) -> Self::Result {
        let peer_id = PeerId::new(msg.user_uuid.clone());

        // If the peer already exists, directly return a completed Future with error
        if self.peers.contains_key(&peer_id) {
            return Box::pin(async move { Err(LobbyError::PeerAlreadyExists()) }.into_actor(self));
        }

        let peer_addr = Peer::new(peer_id.clone(), ctx.address(), msg.role).start();
        self.peers.insert(peer_id, peer_addr.clone());

        let offer = msg.offer.clone();
        let fut = async move {
            let result = peer_addr.send(PeerStartReceiving { offer }).await;

            match result {
                Ok(val) => match val {
                    Ok(answer) => Ok(answer),
                    Err(e) => Err(LobbyError::PeerInternalError(e)),
                },
                Err(e) => Err(LobbyError::MailboxError(e)),
            }
        }
        .into_actor(self);

        Box::pin(fut)
    }
}

#[derive(Message)]
#[rtype(result = " LobbyResult<String>")]
pub struct SubscribeToPeers {
    pub user_uuid: String,
    pub offer: String,
}

impl Handler<SubscribeToPeers> for Lobby {
    type Result = LobbyResult<String>;

    fn handle(&mut self, _msg: SubscribeToPeers, _: &mut Self::Context) -> Self::Result {
        Ok("".to_string())
    }
}

#[allow(dead_code)]
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
    #[allow(dead_code)]
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
            addr.do_send(PeerShutdown {});
        }

        if self.peers.is_empty() {
            self.stop(ctx);
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct PeerStopped {
    pub id: PeerId,
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
