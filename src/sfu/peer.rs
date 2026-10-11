use crate::sfu::config::SfuConfig;
use crate::sfu::error::{PeerError, PeerResult};
use crate::sfu::lobby::{Lobby, PeerStopped};
use crate::sfu::media::{AddMedia, Media, MuteMedia, MuteRemoteMedia, RemoveMedia};
use crate::sfu::rtc::{
    control_data_channel::ControlDataChannel,
    data_channel::{
        start_data_channel_reader, DataChannelMsg, DataChannelReader, EventType, OnDataChannel,
    },
    EndpointBuilder, EndpointEvent, EndpointEventHandler, EndpointId, EndpointKind, PortAllocator,
    PublishEndpoint, SubscribeEndpoint,
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
    subscribe_endpoint: Option<Arc<Mutex<SubscribeEndpoint>>>,

    control_channel: Arc<Mutex<ControlDataChannel>>,
    data_channel_reader: Option<DataChannelReader>,
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
            subscribe_endpoint: None,
            control_channel: Arc::new(Mutex::new(ControlDataChannel::new(id))),
            data_channel_reader: None,
        }
    }

    fn replace_data_channel_reader(&mut self, reader: DataChannelReader) {
        if let Some(reader) = self.data_channel_reader.replace(reader) {
            reader.cancel();
        }
    }

    fn stop_data_channel_reader(&mut self) {
        if let Some(reader) = self.data_channel_reader.take() {
            reader.cancel();
        }
    }
}

impl Actor for Peer {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        log::info!("peer actor peer_id={} is alive", self.id);
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        self.stop_data_channel_reader();
        log::info!("peer actor peer_id={} is stopped", self.id);
    }
}

#[derive(Message)]
#[rtype(result = "PeerResult<String>")]
pub struct PeerCreatePublishAnswer {
    pub offer: String,
}

impl Handler<PeerCreatePublishAnswer> for Peer {
    type Result = ResponseActFuture<Self, PeerResult<String>>;

    fn handle(&mut self, msg: PeerCreatePublishAnswer, ctx: &mut Self::Context) -> Self::Result {
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
                let publish_endpoint = Arc::new(Mutex::new(PublishEndpoint::new(endpoint)));
                let answer = {
                    let mut publish_endpoint = publish_endpoint.lock().await;
                    publish_endpoint.accept_offer(sdp_offer).await?
                };
                Ok::<_, PeerError>((publish_endpoint, answer))
            }
            .into_actor(self)
            .map(|res, actor, _ctx| match res {
                Ok((publish_endpoint, answer)) => {
                    actor.publish_endpoint = Some(publish_endpoint);
                    Ok(answer)
                }
                Err(err) => Err(err),
            }),
        )
    }
}

impl Handler<EndpointEvent> for Peer {
    type Result = ();

    fn handle(&mut self, msg: EndpointEvent, ctx: &mut Self::Context) -> Self::Result {
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
            EndpointEvent::DataChannel {
                endpoint_id,
                data_channel,
            } => {
                log::debug!(
                    "endpoint data channel received, endpoint_id={}",
                    endpoint_id
                );
                let reader = start_data_channel_reader(data_channel, ctx.address(), endpoint_id);
                self.replace_data_channel_reader(reader);
            }
            EndpointEvent::Track { endpoint_id, track } => {
                log::debug!("endpoint track received, endpoint_id={}", endpoint_id);

                if !endpoint_id.as_kind().is_publish() {
                    return;
                }

                let Some(publish_endpoint) = self.publish_endpoint.clone() else {
                    log::warn!(
                        "received publish track without publish endpoint, peer_id={}, endpoint_id={}",
                        self.id,
                        endpoint_id
                    );
                    return;
                };

                let peer_id = self.id.clone();
                let lobby_addr = self.parent_addr.clone();
                actix::spawn(async move {
                    let mut publish_endpoint = publish_endpoint.lock().await;
                    publish_endpoint
                        .receive_track(peer_id, lobby_addr, track)
                        .await;
                });
            }
        }
    }
}

#[derive(Message)]
#[rtype(result = "PeerResult<String>")]
pub struct PeerCreateSubscribeOffer {
    pub medias: Vec<Media>,
}

impl Handler<PeerCreateSubscribeOffer> for Peer {
    // Returns SDP Offer
    type Result = ResponseActFuture<Self, PeerResult<String>>;

    fn handle(&mut self, msg: PeerCreateSubscribeOffer, ctx: &mut Self::Context) -> Self::Result {
        log::info!(
            "setup subscribe endpoint for peer actor peer_id={}",
            self.id
        );
        let endpoint_id = EndpointId::new(self.id.clone(), EndpointKind::Subscribe);
        let handler = Arc::new(EndpointEventHandler::new(
            endpoint_id.clone(),
            ctx.address().recipient(),
        ));
        let builder = EndpointBuilder::from_sfu_config(&self.sfu_config);
        let port_allocator = self.port_allocator.clone();
        let peer_id = self.id.clone();

        Box::pin(
            async move {
                let endpoint = builder.build(endpoint_id, handler, &port_allocator).await?;
                let mut subscribe_endpoint = SubscribeEndpoint::new(endpoint);

                for media in msg.medias {
                    match subscribe_endpoint.add_media(media.clone()).await {
                        Ok(_) => {
                            log::info!("added media to subscribe endpoint, peer_id={}, media_id={}, kind={}", peer_id, media.id, media.kind);
                        }
                        Err(e) => {
                            log::error!(
                                "failed to add media to subscribe endpoint, peer_id={}: {}",
                                peer_id,
                                e
                            );
                        }
                    }
                }

                let offer = subscribe_endpoint.create_offer().await?;
                Ok::<_, PeerError>((Arc::new(Mutex::new(subscribe_endpoint)), offer))
            }
            .into_actor(self)
            .map(|res, actor, _| match res {
                Ok((subscribe_endpoint, offer)) => {
                    actor.subscribe_endpoint = Some(subscribe_endpoint);
                    Ok(offer)
                }
                Err(e) => Err(e),
            }),
        )
    }
}

#[derive(Message)]
#[rtype(result = "PeerResult<String>")]
pub struct PeerReceiveSubscribeAnswer {
    pub answer: String,
}

impl Handler<PeerReceiveSubscribeAnswer> for Peer {
    type Result = ResponseActFuture<Self, PeerResult<String>>;

    fn handle(
        &mut self,
        msg: PeerReceiveSubscribeAnswer,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        log::info!(
            "set subscribe endpoint answer for peer actor peer_id={}",
            self.id
        );

        let sdp_answer = msg.answer;
        let subscribe_endpoint = self.subscribe_endpoint.clone();

        let peer_id = self.id.clone();
        let parent_addr = self.parent_addr.clone();
        Box::pin(
            async move {
                let Some(subscribe_endpoint) = subscribe_endpoint else {
                    log::warn!(
                        "Peer has no subscribe endpoint for answer, peer_id={}",
                        peer_id
                    );
                    return Err(PeerError::MissingSubscribeEndpoint());
                };

                match {
                    let mut subscribe_endpoint = subscribe_endpoint.lock().await;
                    subscribe_endpoint
                        .accept_answer(peer_id.clone(), sdp_answer)
                        .await
                } {
                    Ok(activations) => {
                        for activation in activations {
                            parent_addr.do_send(activation);
                        }
                        Ok("".to_string())
                    }
                    Err(err) => {
                        log::error!("set_answer failed peer_id={}: {:?}", peer_id, err);
                        Err(err.into())
                    }
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

        let Some(subscribe_endpoint_arc) = self.subscribe_endpoint.clone() else {
            return Box::pin(
                async move {
                    log::warn!(
                        "cant add media media_id={} because no subscribe endpoint for peer_id={}",
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
                    let mut subscribe_endpoint = subscribe_endpoint_arc.lock().await;
                    subscribe_endpoint.add_media(media).await
                } {
                    log::error!("failed add media {} for peer {}: {}", media_id, peer_id, e);
                    return None;
                }

                let offer = {
                    let mut subscribe_endpoint = subscribe_endpoint_arc.lock().await;
                    subscribe_endpoint.create_offer().await.ok()
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
        let Some(subscribe_endpoint_arc) = self.subscribe_endpoint.clone() else {
            return Box::pin(
                async move {
                    log::warn!(
                        "cant remove media media_id={} because no subscribe endpoint for peer_id={}",
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
                    let mut subscribe_endpoint = subscribe_endpoint_arc.lock().await;
                    subscribe_endpoint.remove_track(media_id.clone()).await
                } {
                    log::error!(
                        "Failed to remove media media_id={} from subscribe endpoint of peer_id={}: {}",
                        media_id,
                        peer_id,
                        e
                    );
                    return None;
                }
                let offer = {
                    let mut subscribe_endpoint = subscribe_endpoint_arc.lock().await;
                    subscribe_endpoint.create_offer().await.ok()
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

impl Handler<OnDataChannel> for Peer {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: OnDataChannel, _ctx: &mut Self::Context) -> Self::Result {
        let reader_matches_channel = self
            .data_channel_reader
            .as_ref()
            .is_some_and(|reader| reader.matches(&msg.endpoint_id, &msg.dc));
        if matches!(msg.event, EventType::Closed) && reader_matches_channel {
            self.stop_data_channel_reader();
        }

        let dc = msg.dc.clone();
        let endpoint_id = msg.endpoint_id;
        let event = msg.event;
        let control_arc = self.control_channel.clone();

        if !endpoint_id.as_kind().is_publish() {
            return Box::pin(async move {}.into_actor(self));
        }

        Box::pin(
            async move {
                match event {
                    EventType::Open => {
                        log::info!("Publish DataChannel open: attach to control");
                        let mut control = control_arc.lock().await;
                        let _ = control.set_dc(dc).await;
                    }
                    EventType::Closed => {
                        log::info!("Publish DataChannel close: remove from control");
                        let mut control = control_arc.lock().await;
                        control.detach_channel(dc);
                    }
                }
            }
            .into_actor(self),
        )
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
                let Some(subscribe_endpoint_arc) = self.subscribe_endpoint.clone() else {
                    return Box::pin(
                        async move {
                            log::warn!(
                                "Peer has no subscribe endpoint for signal answer for peer_id={}",
                                peer_id
                            );
                        }
                        .into_actor(self),
                    );
                };

                let control_arc = self.control_channel.clone();
                let parent_addr = self.parent_addr.clone();
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

                        match {
                            let mut subscribe_endpoint = subscribe_endpoint_arc.lock().await;
                            subscribe_endpoint.set_signal_answer(msg).await
                        } {
                            Ok(activations) => {
                                for activation in activations {
                                    parent_addr.do_send(activation);
                                }
                            }
                            Err(e) => {
                                log::error!(
                                    "Failed to set signaling answer for peer_id={}: {}",
                                    peer_id,
                                    e
                                );
                            }
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
        let Some(subscribe_endpoint_arc) = self.subscribe_endpoint.clone() else {
            return Box::pin(
                async move {
                    log::warn!(
                        "Peer has no subscribe endpoint for send remote mute, peer_id={}",
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
                    let mut subscribe_endpoint = subscribe_endpoint_arc.lock().await;
                    subscribe_endpoint
                        .get_mid_and_mute(media_id.clone(), mute)
                        .await
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
        self.stop_data_channel_reader();

        let peer_id = self.id.clone();
        let parent_addr = self.parent_addr.clone();
        let subscribe_endpoint = self.subscribe_endpoint.clone();
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

                if let Some(subscribe_endpoint_arc) = subscribe_endpoint {
                    {
                        let subscribe_endpoint = subscribe_endpoint_arc.lock().await;
                        let _ = subscribe_endpoint.shutdown().await;
                    }
                }

                parent_addr.do_send(PeerStopped {
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
