use crate::sfu::error::{PeerError, PeerResult};
use crate::sfu::lobby::{Lobby, PeerStopped};
use crate::sfu::media::connector::ConnectorType;
use crate::sfu::media::data_channel::{DataChannelMsg, OnDataChannel};
use crate::sfu::media::message::MediaMessage;
use crate::sfu::media::receiver::Receiver;
use crate::sfu::media::sender::Sender;
use crate::sfu::media::{AddMedia, Media, RemoveMedia};
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
}

impl Actor for Peer {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        log::info!("peer actor peer_id={} is alive", self.id);
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        log::info!("peer actor peer_id={} is stopped", self.id);
    }
    // fn stopping(&mut self, _ctx: &mut Context<Self>) -> Running {
    //     log::info!("peer actor peer_id={} is stopping", self.id);
    //     let _ = self.cleanup().await;
    //     Running::Stop
    // }
}

#[derive(Message)]
#[rtype(result = "PeerResult<String>")]
pub struct PeerStartReceiving {
    pub offer: String,
}

impl Handler<PeerStartReceiving> for Peer {
    type Result = ResponseActFuture<Self, PeerResult<String>>;

    fn handle(&mut self, msg: PeerStartReceiving, ctx: &mut Self::Context) -> Self::Result {
        log::info!("starting (Receiver) for peer actor peer_id={}", self.id);
        let id = self.id.clone();
        let addr = ctx.address();
        let lobby_addr = self.parent_addr.clone();
        let sdp_offer = msg.offer;

        // Prepare the Future
        Box::pin(
            async move {
                let mut receiver = Receiver::new(id, addr, lobby_addr).await?;
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
    pub medias: Vec<Media>,
}

impl Handler<PeerStartSending> for Peer {
    type Result = ResponseActFuture<Self, PeerResult<String>>;

    fn handle(&mut self, msg: PeerStartSending, ctx: &mut Self::Context) -> Self::Result {
        log::info!("setup (Sender) for peer actor peer_id={}", self.id);
        let id = self.id.clone();
        let addr = ctx.address();
        let sdp_offer = msg.offer;

        // Prepare the Future
        Box::pin(
            async move {
                let mut sender = Sender::new(id.clone(), addr).await?;
                for media in msg.medias {
                    let track = media.subscribe();
                    if let Err(e) = sender.add_track(track).await {
                        log::error!(
                            "On subscribe (Sender), failed to add track to peer_id={} : {}",
                            id,
                            e
                        );
                    }
                }
                let answer = sender.connect(sdp_offer.as_str()).await?;
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

impl Handler<AddMedia> for Peer {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: AddMedia, _ctx: &mut Self::Context) -> Self::Result {
        let peer_id = self.id.clone();
        let media_id = msg.media.id.clone();
        let Some(mut sender) = self.sender.clone() else {
            return Box::pin(
                async move {
                    log::warn!(
                        "cant add media media_id={} because no (Sender) for peer_id={}",
                        media_id,
                        peer_id
                    );
                }
                .into_actor(self),
            );
        };

        let media = msg.media;
        Box::pin(
            async move {
                let track = media.subscribe();
                if let Err(e) = sender.add_track(track).await {
                    log::error!(
                        "On subscribe, failed to add media media_id={} to sender of peer_id={}: {}",
                        media_id,
                        peer_id,
                        e
                    );
                }
                if let Err(e) = sender.send_signaling_offer().await {
                    log::error!(
                        "On add media, failed send offer media_id={} by (Sender) of peer_id={}: {}",
                        media_id,
                        peer_id,
                        e
                    );
                }
            }
            .into_actor(self),
        )
    }
}

impl Handler<RemoveMedia> for Peer {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: RemoveMedia, _ctx: &mut Self::Context) -> Self::Result {
        let peer_id = self.id.clone();
        let media_id = msg.media_id;
        let Some(mut sender) = self.sender.clone() else {
            return Box::pin(
                async move {
                    log::warn!(
                        "cant remove media media_id={} because no (Sender) for peer_id={}",
                        media_id,
                        peer_id
                    );
                }
                .into_actor(self),
            );
        };

        Box::pin(
            async move {
                if let Err(e) = sender.remove_track(media_id.to_string()).await {
                    log::error!(
                        "Failed to remove media media_id={} from sender of peer_id={}: {}",
                        media_id,
                        peer_id,
                        e
                    );
                }
                if let Err(e) = sender.send_signaling_offer().await {
                    log::error!(
                        "On remove media, failed send offer media_id={} by (Sender) of peer_id={}: {}",
                        media_id,
                        peer_id,
                        e
                    );
                }
            }
            .into_actor(self),
        )
    }
}

impl Handler<MediaMessage> for Peer {
    type Result = ();

    fn handle(&mut self, msg: MediaMessage, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            MediaMessage::Connected(connector_type) => {
                let peer_id = self.id.clone();
                log::info!(
                    "media connected, type={}, peer_id={}",
                    connector_type,
                    peer_id
                );
            }

            MediaMessage::Disconnected(connector_type) => {
                let peer_id = self.id.clone();
                log::info!(
                    "media disconnected,  type={}, peer_id={}",
                    connector_type,
                    peer_id
                );
                _ctx.notify(PeerShutdown {});
            }
            _ => (),
        }
    }
}

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
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: DataChannelMsg, _ctx: &mut Self::Context) -> Self::Result {
        let peer_id = self.id.clone();
        match msg {
            DataChannelMsg::OfferMsg(msg) => {
                let Some(mut receiver) = self.receiver.clone() else {
                    return Box::pin(
                        async move {
                            log::warn!(
                                "Peer has no sender for signal answer for peer_id={}",
                                peer_id
                            );
                        }
                        .into_actor(self),
                    );
                };
                Box::pin(
                    async move {
                        if let Err(e) = receiver.on_signaling_offer(msg).await {
                            log::error!(
                                "Failed to set signaling offer for peer_id={}: {}",
                                peer_id,
                                e
                            );
                        }
                    }
                    .into_actor(self),
                )
            }
            DataChannelMsg::AnswerMsg(msg) => {
                let Some(mut sender) = self.sender.clone() else {
                    return Box::pin(
                        async move {
                            log::warn!(
                                "Peer has no sender for signal answer for peer_id={}",
                                peer_id
                            );
                        }
                        .into_actor(self),
                    );
                };

                Box::pin(
                    async move {
                        if let Err(e) = sender.on_signaling_answer(msg).await {
                            log::error!(
                                "Failed to set signaling answer for peer_id={}: {}",
                                peer_id,
                                e
                            );
                        }
                    }
                    .into_actor(self),
                )
            }
            DataChannelMsg::MuteMsg(_) => Box::pin(async move {}.into_actor(self)),
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct PeerShutdown {}

impl Handler<PeerShutdown> for Peer {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, _msg: PeerShutdown, _ctx: &mut Self::Context) -> Self::Result {
        log::info!("shutting down peer actor peer_id={}", self.id);

        let peer_id = self.id.clone();
        let parent_addr = self.parent_addr.clone();
        let sender = self.sender.clone();
        let receiver = self.receiver.clone();

        Box::pin(
            async move {
                log::info!("cleanup peer actor, peer_id={}", peer_id);
                if let Some(receiver) = receiver {
                    let _ = receiver.shutdown().await;
                }

                if let Some(sender) = sender {
                    let _ = sender.shutdown().await;
                }

                let _ = parent_addr.try_send(PeerStopped {
                    id: peer_id.clone(),
                });
            }
            .into_actor(self)
            .map(|_, actor, ctx| {
                log::info!("stop peer actor, peer_id={}", actor.id);
                ctx.stop();
            }),
        )
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
