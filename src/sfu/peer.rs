use crate::sfu::error::{PeerError, PeerResult};
use crate::sfu::lobby::{Lobby, PeerStopped};
use crate::sfu::media::connector::{Connector, ConnectorType};
use crate::sfu::media::data_channel::{DataChannelMsg, OnDataChannel};
use crate::sfu::media::message::MediaMessage;
use crate::sfu::media::receiver::Receiver;
use crate::sfu::media::sender::Sender;
use actix::{Actor, ActorContext, Addr, AsyncContext, Context, Handler, Message, WrapFuture};
use actix::{ActorFutureExt, ResponseActFuture};
use derive_more::Display;

pub struct Peer {
    pub id: PeerId,
    #[allow(dead_code)]
    pub role: PeerRole,
    parent_addr: Addr<Lobby>,
    receiver: Option<Receiver>,
    sender: Option<Sender>,
}

impl Peer {
    pub fn new(id: PeerId, parent_addr: Addr<Lobby>, role: PeerRole) -> Self {
        Self {
            id,
            role,
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

    fn started(&mut self, _ctx: &mut Context<Self>) {
        log::info!("started: peer actor peer_id={} is alive", self.id);
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        log::info!("stopped: peer actor peer_id={} is stopped", self.id);
    }
}

#[derive(Message)]
#[rtype(result = "PeerResult<String>")]
pub struct PeerStartReceiving {
    pub offer: String,
}

impl Handler<PeerStartReceiving> for Peer {
    type Result = ResponseActFuture<Self, PeerResult<String>>;

    fn handle(&mut self, msg: PeerStartReceiving, ctx: &mut Self::Context) -> Self::Result {
        log::info!("star receiving for peer actor peer_id={} is alive", self.id);
        let id = self.id.clone();
        let addr = ctx.address();
        let sdp_offer = msg.offer;

        // Prepare the Future
        Box::pin(
            async move {
                let mut receiver = Receiver::new(id, addr).await?;
                let answer = receiver.connect(sdp_offer.as_str()).await?;
                Ok((receiver, answer))
            }
            .into_actor(self)
            .map(|res, actor, _| match res {
                Ok((receiver, answer)) => {
                    actor.receiver = Some(receiver);
                    Ok(answer)
                }
                Err(e) => Err(PeerError::InternalMedia(e)),
            }),
        )
    }
}

#[derive(Message)]
#[rtype(result = "PeerResult<String>")]
pub struct PeerStartSending {
    pub offer: String,
}

impl Handler<PeerStartSending> for Peer {
    type Result = ResponseActFuture<Self, PeerResult<String>>;

    fn handle(&mut self, msg: PeerStartSending, ctx: &mut Self::Context) -> Self::Result {
        log::info!("star receiving for peer actor peer_id={} is alive", self.id);
        let id = self.id.clone();
        let addr = ctx.address();
        let sdp_offer = msg.offer;

        // Prepare the Future
        Box::pin(
            async move {
                let sender = Sender::new(id, addr).await?;
                let answer = sender.create_answer(sdp_offer.as_str()).await?;
                Ok((sender, answer))
            }
            .into_actor(self)
            .map(|res, actor, _| match res {
                Ok((sender, answer)) => {
                    actor.sender = Some(sender);
                    Ok(answer)
                }
                Err(e) => Err(PeerError::InternalMedia(e)),
            }),
        )
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

#[derive(Debug, Clone)]
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

    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for PeerId {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}
