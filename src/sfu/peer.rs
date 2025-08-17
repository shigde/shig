use crate::sfu::lobby::{Lobby, PeerStopped};
use crate::sfu::media::connector::ConnectorType;
use crate::sfu::media::data_channel::{DataChannelMsg, OnDataChannel};
use crate::sfu::media::message::MediaMessage;
use crate::sfu::media::receiver::Receiver;
use crate::sfu::media::sender::Sender;
use actix::ActorFutureExt;
use actix::{Actor, ActorContext, Addr, AsyncContext, Context, Handler, Message, WrapFuture};
use derive_more::Display;

pub struct Peer {
    pub id: PeerId,
    parent_addr: Addr<Lobby>,
    receiver: Option<Receiver>,
    sender: Option<Sender>,
}

impl Peer {
    pub fn new(id: PeerId, parent_addr: Addr<Lobby>) -> Self {
        Self {
            id,
            parent_addr,
            receiver: None,
            sender: None,
        }
    }

    fn stop(&self, ctx: &mut Context<Peer>) {
        self.parent_addr.do_send(PeerStopped {
            id: self.id.clone(),
        });
        ctx.stop();
    }
}

impl Actor for Peer {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        log::info!("started: peer actor peer_id={} is alive", self.id);
        let id = self.id.clone();
        let addr = ctx.address();

        ctx.spawn(
            async move { Receiver::new(id, addr).await }
                .into_actor(self)
                .map(|receiver, actor, _ctx| match receiver {
                    Ok(r) => {
                        actor.receiver = Some(r);
                        log::info!("Receiver successfully created");
                    }
                    Err(e) => {
                        log::error!("Receiver could not be created: {:?}", e);
                    }
                }),
        );

        let peer_id = self.id.clone();
        let peer_addr = ctx.address();
        ctx.spawn(
            async move { Sender::new(peer_id, peer_addr).await }
                .into_actor(self)
                .map(|sender, actor, _ctx| match sender {
                    Ok(s) => {
                        actor.sender = Some(s);
                        log::info!("Sender successfully created");
                    }
                    Err(e) => {
                        log::error!("Sender could not be created: {:?}", e);
                    }
                }),
        );
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        log::info!("stopped: peer actor peer_id={} is stopped", self.id);
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

impl Handler<MediaMessage> for Peer {
    type Result = ();

    fn handle(&mut self, msg: MediaMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            MediaMessage::Connected(_t) => {}
            MediaMessage::Disconnected(_t) => {
                self.stop(ctx);
            }
            _ => {}
        }
    }
}

// https://blog.lminiero.it/live-performance/

impl Handler<OnDataChannel> for Peer {
    type Result = ();

    fn handle(&mut self, msg: OnDataChannel, _ctx: &mut Self::Context) -> Self::Result {
        match msg.kind {
            ConnectorType::Sender => {}
            ConnectorType::Receiver => {}
        }
    }
}

impl Handler<DataChannelMsg> for Peer {
    type Result = ();

    fn handle(&mut self, msg: DataChannelMsg, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            DataChannelMsg::OfferMsg(_) => {
                // update receiver
            }
            DataChannelMsg::AnswerMsg(_) => {
                // update sender
            }
            DataChannelMsg::MuteMsg(_) => {
                // ..
            }
        }
    }
}

pub enum PeerRole {
    Host,
    Guest,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Display)]
pub struct PeerId(String);

impl PeerId {
    pub fn new<S: Into<String>>(s: S) -> Self {
        PeerId(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for PeerId {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}
