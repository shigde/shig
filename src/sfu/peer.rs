use crate::sfu::config::SfuConfig;
use crate::sfu::error::{PeerError, PeerResult};
use crate::sfu::lobby::{LeavePeer, Lobby, PeerStopped};
use crate::sfu::media::connector::{Connector, ConnectorType};
use crate::sfu::media::control_data_channel::ControlDataChannel;
use crate::sfu::media::data_channel::{DataChannelMsg, EventType, OnDataChannel};
use crate::sfu::media::message::MediaMessage;
use crate::sfu::media::sender::Sender;
use crate::sfu::media::{AddMedia, Media, MuteMedia, MuteRemoteMedia, RemoveMedia};
use crate::sfu::rtc::{
    EndpointBuilder, EndpointEvent, EndpointEventHandler, EndpointId, EndpointKind, PortAllocator,
    PublishEndpoint,
};
use actix::{Actor, ActorContext, Addr, AsyncContext, Context, Handler, Message, WrapFuture};
use actix::{ActorFutureExt, ResponseActFuture};
use derive_more::Display;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Peer {
    pub id: PeerId,
    #[allow(dead_code)]
    pub role: PeerRole,
    sfu_config: SfuConfig,
    port_allocator: PortAllocator,
    parent_addr: Addr<Lobby>,
    publish_endpoint: Option<Arc<Mutex<PublishEndpoint>>>,
    sender: Option<Arc<Mutex<Sender>>>,
    control_channel: Arc<Mutex<ControlDataChannel>>,
}

impl Peer {
    pub fn new(
        id: PeerId,
        parent_addr: Addr<Lobby>,
        role: PeerRole,
        sfu_config: SfuConfig,
        port_allocator: PortAllocator,
    ) -> Self {
        Self {
            id: id.clone(),
            role,
            parent_addr,
            sfu_config,
            port_allocator,
            publish_endpoint: None,
            sender: None,
            control_channel: Arc::new(Mutex::new(ControlDataChannel::new(id))),
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
pub struct PeerOfferForPublishEndpoint {
    pub offer: String,
}

impl Handler<PeerOfferForPublishEndpoint> for Peer {
    type Result = ResponseActFuture<Self, PeerResult<String>>;

    fn handle(&mut self, msg: PeerOfferForPublishEndpoint, ctx: &mut Self::Context) -> Self::Result {
        log::info!("Init (Publish) for peer actor peer_id={}", self.id);
        let endpoint_id = EndpointId::new(self.id.clone(), EndpointKind::Publish);
        let handler = Arc::new(EndpointEventHandler::new(
            endpoint_id.clone(),
            ctx.address().recipient(),
        ));
        let builder = EndpointBuilder::from_sfu_config(&self.sfu_config);
        let port_allocator = self.port_allocator.clone();
        let sdp_offer = msg.offer;

        Box::pin(
            async move {
                let endpoint = builder.build(endpoint_id, handler, &port_allocator).await?;
                Ok::<_, PeerError>(Arc::new(Mutex::new(PublishEndpoint::new(endpoint))))
            }
            .into_actor(self)
            .then(|res, actor, _ctx| match res {
                Ok(publish_endpoint) => {
                    actor.publish_endpoint = Some(publish_endpoint.clone());
                    async move {
                        let publish_endpoint = publish_endpoint.lock().await;
                        publish_endpoint.accept_offer(sdp_offer).await.map_err(Into::into)
                    }
                    .into_actor(actor)
                }
                Err(err) => async move { Err(err) }.into_actor(actor),
            }),
        )
    }
}

impl Handler<EndpointEvent> for Peer {
    type Result = ();

    fn handle(&mut self, msg: EndpointEvent, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            EndpointEvent::NegotiationNeeded { endpoint_id } => {
                log::debug!("endpoint negotiation needed, endpoint_id={}", endpoint_id);
            }
            EndpointEvent::IceCandidate { endpoint_id, .. } => {
                log::debug!("endpoint ice candidate, endpoint_id={}", endpoint_id);
            }
            EndpointEvent::IceCandidateError { endpoint_id, event } => {
                log::warn!(
                    "endpoint ice candidate error, endpoint_id={}, error={:?}",
                    endpoint_id,
                    event
                );
            }
            EndpointEvent::SignalingStateChange { endpoint_id, state } => {
                log::debug!(
                    "endpoint signaling state changed, endpoint_id={}, state={:?}",
                    endpoint_id,
                    state
                );
            }
            EndpointEvent::IceConnectionStateChange { endpoint_id, state } => {
                log::debug!(
                    "endpoint ice connection state changed, endpoint_id={}, state={:?}",
                    endpoint_id,
                    state
                );
            }
            EndpointEvent::IceGatheringStateChange { endpoint_id, state } => {
                log::debug!(
                    "endpoint ice gathering state changed, endpoint_id={}, state={:?}",
                    endpoint_id,
                    state
                );
            }
            EndpointEvent::ConnectionStateChange { endpoint_id, state } => {
                log::info!(
                    "endpoint connection state changed, endpoint_id={}, state={:?}",
                    endpoint_id,
                    state
                );
            }
            EndpointEvent::DataChannel { endpoint_id, .. } => {
                log::debug!("endpoint data channel received, endpoint_id={}", endpoint_id);
            }
            EndpointEvent::Track { endpoint_id, .. } => {
                log::debug!("endpoint track received, endpoint_id={}", endpoint_id);
            }
        }
    }
}

#[derive(Message)]
#[rtype(result = "PeerResult<String>")]
pub struct PeerStartSending {
    pub medias: Vec<Media>,
}

impl Handler<PeerStartSending> for Peer {
    // Returns SDP Answer
    type Result = ResponseActFuture<Self, PeerResult<String>>;

    fn handle(&mut self, msg: PeerStartSending, ctx: &mut Self::Context) -> Self::Result {
        log::info!("setup (Sender) for peer actor peer_id={}", self.id);
        let id = self.id.clone();
        let addr = ctx.address();

        // Prepare the Future
        let sfu_config = self.sfu_config.clone();
        Box::pin(
            async move {
                let mut sender = Sender::new(id.clone(), addr, sfu_config).await?;
                for media in msg.medias {
                    match sender.add_media(media.clone()).await {
                        Ok(_) => {
                            log::info!("On subscribe (Sender), added media to peer_id={}, media_id= {}, kind={}", id, media.id, media.kind);
                        }
                        Err(e) => {
                            log::error!(
                                "On subscribe (Sender), failed to add media to peer_id={} : {}",
                                id,
                                e
                            );
                        }
                    }
                }
                let answer = sender.setup_offer().await?;
                Ok((sender, answer))
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

        let peer_id = self.id.clone();
        Box::pin(
            async move {
                if let Err(err) = {
                    let sender = sender_arc.lock().await;
                    sender.set_answer(sdp_answer.as_str()).await
                } {
                    log::error!("set_answer failed peer_id={}: {:?}", peer_id, err);
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
        let control_arc = self.control_channel.clone();

        Box::pin(
            async move {
                if let Err(e) = {
                    let mut sender = sender_arc.lock().await;
                    sender.add_media(media).await
                } {
                    log::error!("failed add media {} for peer {}: {}", media_id, peer_id, e);
                    return None;
                }

                let offer = {
                    let mut sender = sender_arc.lock().await;
                    sender.create_signal_offer().await.ok()
                };
                offer
            }
            .into_actor(self)
            .then(|offer_opt, actor, _ctx| {
                async move {
                    if let Some(offer) = offer_opt {
                        let mut control = control_arc.lock().await;
                        let _ = control.send_offer(offer).await;
                    }
                }
                .into_actor(actor)
            }),
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
        let control_arc = self.control_channel.clone();
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
                    return None;
                }
                let offer = {
                    let mut sender = sender_arc.lock().await;
                    sender.create_signal_offer().await.ok()
                };
                offer
            }
            .into_actor(self)
            .then(|offer_opt, actor, _ctx| {
                async move {
                    if let Some(offer) = offer_opt {
                        let mut control = control_arc.lock().await;
                        let _ = control.send_offer(offer).await;
                    }
                }
                .into_actor(actor)
            }),
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
                self.parent_addr.do_send(LeavePeer {
                    user_uuid: self.id.as_user_uuid(),
                });
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
        let event = msg.event;
        let control_arc = self.control_channel.clone();
        match kind {
            // ignore the sender dc
            ConnectorType::Sender => Box::pin(async move {}.into_actor(self)),
            ConnectorType::Receiver => Box::pin(
                async move {
                    match event {
                        EventType::Open => {
                            log::info!("Receiver DataChannel open: attach to control");
                            let mut control = control_arc.lock().await;
                            let _ = control.set_dc(dc).await;
                        }
                        EventType::Closed => {
                            log::info!("Receiver DataChannel close: remove from control");
                            let mut control = control_arc.lock().await;
                            let _ = control.detach_channel(dc);
                        }
                    }
                }
                .into_actor(self),
            ),
        }
    }
}

impl Handler<DataChannelMsg> for Peer {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: DataChannelMsg, _ctx: &mut Self::Context) -> Self::Result {
        let peer_id = self.id.clone();
        match msg {
            DataChannelMsg::OfferMsg(msg) => {
                Box::pin(
                    async move {
                        log::warn!(
                            "Peer cannot handle signal offer with publish endpoint yet, peer_id={}, offer_number={}",
                            peer_id,
                            msg.number
                        );
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

                let control_arc = self.control_channel.clone();
                Box::pin(
                    async move {
                        let is_answer_stale = {
                            let control = control_arc.lock().await;
                            control.is_answer_stale(msg.number)
                        };

                        if is_answer_stale {
                            log::info!("ignore staled answer peer_id={}", peer_id);
                            return;
                        }

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

        let control_arc = self.control_channel.clone();
        let mute = msg.mute;
        let media_id = msg.media_id;
        log::info!(
            "send mute remote signal for peer_id={} media_id={}",
            peer_id.clone(),
            media_id.clone()
        );
        Box::pin(
            async move {
                let mid_option = {
                    let mut sender = sender_arc.lock().await;
                    sender.get_mid_and_mute(media_id.clone(), mute)
                };

                let Some(mid) = mid_option else {
                    log::warn!(
                        "no mid for peer_id={} media_id={}",
                        peer_id.clone(),
                        media_id.clone()
                    );
                    return;
                };

                if let Err(e) = {
                    let mut control = control_arc.lock().await;
                    control.send_mute(mid.as_str(), mute).await
                } {
                    log::error!(
                        "Failed to send remote mute for peer_id={} media_id={}: {}",
                        peer_id,
                        media_id,
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
        let publish_endpoint = self.publish_endpoint.clone();

        Box::pin(
            async move {
                log::info!("cleanup peer actor, peer_id={}", peer_id);
                if let Some(publish_endpoint_arc) = publish_endpoint {
                    {
                        let publish_endpoint = publish_endpoint_arc.lock().await;
                        let _ = publish_endpoint.shutdown().await;
                    }
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
    pub fn new<S: Into<String>>(user_uuid: S) -> Self {
        PeerId(user_uuid.into())
    }

    pub fn as_user_uuid(&self) -> String {
        self.0.to_string()
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
