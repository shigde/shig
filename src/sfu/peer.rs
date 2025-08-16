use crate::sfu::lobby::{Lobby, PeerStopped};
use crate::sfu::media::data_channel::DataChannelMsg;
use crate::sfu::media::message::MediaMessage;
use actix::{Actor, ActorContext, Addr, Context, Handler, Message};
use derive_more::Display;

pub struct Peer {
    pub id: PeerId,
    parent_addr: Addr<Lobby>,
}

impl Peer {
    pub fn new(id: PeerId, parent_addr: Addr<Lobby>) -> Self {
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
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        log::info!("started: peer actor peer_id={} is alive", self.id);
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
