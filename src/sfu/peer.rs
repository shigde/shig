use crate::sfu::error::{PeerError, PeerResult};
use crate::sfu::lobby::{Lobby, PeerStopped};
use crate::sfu::media::connector::{Connector, ConnectorType};
use crate::sfu::media::control_channel::ControlChannel;
use crate::sfu::media::data_channel::{DataChannel, DataChannelMsg, OnDataChannel};
use crate::sfu::media::message::MediaMessage;
use crate::sfu::media::receiver::Receiver;
use crate::sfu::media::sender::Sender;
use crate::sfu::media::{AddMedia, Media, MuteMedia, MuteRemoteMedia, RemoveMedia};
use actix::{Actor, ActorContext, Addr, AsyncContext, Context, Handler, Message, WrapFuture};
use actix::{ActorFutureExt, ResponseActFuture};
use derive_more::Display;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Peer {
    pub id: PeerId,
    #[allow(dead_code)]
    pub role: PeerRole,
    parent_addr: Addr<Lobby>,
    receiver: Option<Receiver>,
    sender: Option<Arc<Mutex<Sender>>>,
    control_channel: Arc<ControlChannel>,
}

impl Peer {
    pub fn new(id: PeerId, parent_addr: Addr<Lobby>, role: PeerRole) -> Self {
        Self {
            id,
            role,
            parent_addr,
            receiver: None,
            sender: None,
            control_channel: ControlChannel::new(),
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
    pub medias: Vec<Media>,
}

impl Handler<PeerStartSending> for Peer {
    type Result = ResponseActFuture<Self, PeerResult<String>>;

    fn handle(&mut self, msg: PeerStartSending, ctx: &mut Self::Context) -> Self::Result {
        log::info!("setup (Sender) for peer actor peer_id={}", self.id);
        let id = self.id.clone();
        let addr = ctx.address();

        // Prepare the Future
        Box::pin(
            async move {
                let mut sender = Sender::new(id.clone(), addr).await?;
                for media in msg.medias {
                    match sender.add_media(media.clone()).await {
                        Ok(_) => {
                            log::info!("On subscribe (Sender), added media to peer_id={}, media_id= {}, kind={}", id, media.id, media.kind);
                        },
                        Err(e) => {
                            log::error!(
                                "On subscribe (Sender), failed to add media to peer_id={} : {}",
                                id,
                                e
                            );
                        }
                    }
                }
                let offer = sender.setup_offer().await?;
                Ok((sender, offer))
            }
            .into_actor(self)
            .map(|res, actor, _| match res {
                Ok((sender, answer)) => {
                    actor.sender = Some(Arc::new(Mutex::new(sender)));
                    Ok(answer)
                }
                Err(e) => Err(PeerError::InternalMedia(e)),
            }),
        )
    }
}

#[derive(Message)]
#[rtype(result = "PeerResult<String>")]
pub struct PeerSending {
    pub answer: String,
}

impl Handler<PeerSending> for Peer {
    type Result = ResponseActFuture<Self, PeerResult<String>>;

    fn handle(&mut self, msg: PeerSending, _ctx: &mut Self::Context) -> Self::Result {
        log::info!("setup (Sender) for peer actor peer_id={}", self.id);

        let sdp_answer = msg.answer;
        let sender_arc = self.sender.clone().unwrap();

        // add the receiver dc to the sender signaler, because we're doing signaling over the receiver channel
        let receiver_dc = self.receiver.clone().unwrap().get_dc();
        let peer_id = self.id.clone();

        Box::pin(
            async move {
                if let Some(dc) = receiver_dc {
                    log::info!("adding receiver dc to sender signaler, peer_id={}", peer_id);
                    // self.control_channel
                    //     .send("signal-offer", serde_json::to_value(offer)?)
                    //     .await;

                    //if dc.ready_state() == RTCDataChannelState::Open {
                    //     {
                    //         let mut sender = sender_arc.lock().await;
                    //         sender.set_signal_dc(dc).await;
                    //     }
                    //} else {
                    //    log::warn!("receiver dc not ready, peer_id={}", peer_id);
                    //    return Err(PeerError::InternalMedia(anyhow!("receiver dc not ready")));
                }

                if let Err(err) = {
                    let sender = sender_arc.lock().await;
                    sender.set_answer(sdp_answer.as_str()).await
                } {
                    log::error!("set_answer failed: {:?}", err);
                    Err(err.into())
                } else {
                    Ok("".to_string())
                }
            }
            .into_actor(self),
        )
    }
}

impl Handler<AddMedia> for Peer {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: AddMedia, _ctx: &mut Self::Context) -> Self::Result {
        let peer_id = self.id.clone();
        let media_id = msg.media.id.clone();
        let Some(sender_arc) = self.sender.clone() else {
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
                if let Err(e) = {
                    let mut sender = sender_arc.lock().await;
                    sender.add_media(media).await
                } {
                    log::error!(
                        "On add media, failed to add media media_id={} to sender of peer_id={}: {}",
                        media_id,
                        peer_id,
                        e
                    );
                    return;
                }

                if let Err(e) = {
                    let mut sender = sender_arc.lock().await;
                    sender.create_signal_offer().await
                } {
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
        let Some(sender_arc) = self.sender.clone() else {
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
                if let Err(e) = {
                    let mut sender = sender_arc.lock().await;
                    sender.remove_track(media_id.clone()).await
                } {
                    log::error!(
                        "Failed to remove media media_id={} from sender of peer_id={}: {}",
                        media_id,
                        peer_id,
                        e
                    );
                }
                if let Err(e) = {
                    let mut sender = sender_arc.lock().await;
                    sender.create_signal_offer().await
                } {
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
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: OnDataChannel, _ctx: &mut Self::Context) -> Self::Result {
        let dc = msg.dc.clone();
        let kind = msg.kind;
        let messenger = self.control_channel.clone();
        match kind {
            ConnectorType::Sender => Box::pin(async move {}.into_actor(self)),
            ConnectorType::Receiver => {
                Box::pin(
                    async move {
                        log::info!("Receiver DataChannel arrived: attach to control_channe");

                        // 🔗 Messenger an neuen DC binden
                        crate::sfu::media::control_channel::bind_datachannel(dc, messenger).await;
                    }
                    .into_actor(self),
                )
                // let sender_arc_opt = self.sender.clone();
                // Box::pin(
                //     async move {
                //         if let Some(sender_arc) = sender_arc_opt {
                //             {
                //                 let mut sender = sender_arc.lock().await;
                //                 sender.set_signal_dc(dc).await;
                //             }
                //         }
                //     }
                //     .into_actor(self),
                // )
            }
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
                let Some(sender_arc) = self.sender.clone() else {
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
                        if let Err(e) = {
                            let mut sender = sender_arc.lock().await;
                            sender.set_signal_answer(msg).await
                        } {
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
            DataChannelMsg::MuteMsg(mute_data) => {
                let parent = self.parent_addr.clone();
                let peer_id = self.id.clone();
                let mid = mute_data.mid.clone();
                let mute = mute_data.mute;
                Box::pin(
                    async move {
                        parent.do_send(MuteMedia { peer_id, mid, mute });
                    }
                    .into_actor(self),
                )
            }
        }
    }
}

impl Handler<MuteRemoteMedia> for Peer {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: MuteRemoteMedia, _ctx: &mut Self::Context) -> Self::Result {
        let peer_id = self.id.clone();
        let Some(sender_arc) = self.sender.clone() else {
            return Box::pin(
                async move {
                    log::warn!(
                        "Peer has no sender for send remote mute, peer_id={}",
                        peer_id
                    );
                }
                .into_actor(self),
            );
        };

        let mute = msg.mute;
        let media_id = msg.media_id;
        log::info!("send mute remote signal for peer_id={}", peer_id);
        Box::pin(
            async move {
                if let Err(e) = {
                    let mut sender = sender_arc.lock().await;
                    sender.send_mute_remote(media_id, mute).await
                } {
                    log::error!(
                        "Failed to set send remote mute for peer_id={}: {}",
                        peer_id,
                        e
                    );
                }
            }
            .into_actor(self),
        )
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

                if let Some(sender_arc) = sender {
                    {
                        let sender = sender_arc.lock().await;
                        let _ = sender.shutdown().await;
                    }
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
