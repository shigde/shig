use crate::sfu::endpoint::EndpointId;
use crate::sfu::error::{PeerError, PeerResult};
use crate::sfu::lobby::{
    Lobby, PeerStopped, PublishEndpointSucceeded, SubscriptionEndpointSucceeded,
};
use crate::sfu::rtc::core_actor::RtcCoreActor;
use crate::sfu::rtc::media_command::{
    ApplyEndpointAnswer, CloseEndpoint, CreateEndpointOffer, NegotiateEndpoint,
};
use actix::{
    Actor, ActorContext, ActorFutureExt, Addr, Context, Handler, Message, ResponseActFuture,
    WrapFuture,
};
use derive_more::Display;

/// Control-plane actor for one logical participant.
pub struct Peer {
    pub id: PeerId,
    #[allow(dead_code)]
    pub role: PeerRole,
    publish_endpoint: EndpointId,
    subscribe_endpoint: EndpointId,
    rtc_core: Addr<RtcCoreActor>,
    parent_addr: Addr<Lobby>,
}

impl Peer {
    pub fn new(
        id: PeerId,
        parent_addr: Addr<Lobby>,
        role: PeerRole,
        publish_endpoint: EndpointId,
        subscribe_endpoint: EndpointId,
        rtc_core: Addr<RtcCoreActor>,
    ) -> Self {
        Self {
            id,
            role,
            publish_endpoint,
            subscribe_endpoint,
            rtc_core,
            parent_addr,
        }
    }

    fn negotiate(
        &self,
        endpoint_id: EndpointId,
        offer: String,
    ) -> ResponseActFuture<Self, PeerResult<String>> {
        let rtc_core = self.rtc_core.clone();
        Box::pin(
            async move {
                rtc_core
                    .send(NegotiateEndpoint { endpoint_id, offer })
                    .await
                    .map_err(|error| PeerError::Rtc(error.to_string()))?
                    .map_err(PeerError::from)
            }
            .into_actor(self),
        )
    }
}

impl Actor for Peer {
    type Context = Context<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        log::info!("peer actor peer_id={} is alive", self.id);
    }
}

#[derive(Message)]
#[rtype(result = "PeerResult<String>")]
pub struct CreatePublishEndpoint {
    pub offer: String,
}

impl Handler<CreatePublishEndpoint> for Peer {
    type Result = ResponseActFuture<Self, PeerResult<String>>;
    fn handle(&mut self, msg: CreatePublishEndpoint, _ctx: &mut Self::Context) -> Self::Result {
        let endpoint_id = self.publish_endpoint.clone();
        Box::pin(
            self.negotiate(endpoint_id.clone(), msg.offer)
                .map(move |result, actor, _ctx| {
                    if result.is_ok() {
                        actor.parent_addr.do_send(PublishEndpointSucceeded {
                            peer_id: actor.id.clone(),
                            endpoint_id,
                        });
                    }
                    result
                }),
        )
    }
}

#[derive(Message)]
#[rtype(result = "PeerResult<String>")]
pub struct CreateSubscriptionEndpoint {}

impl Handler<CreateSubscriptionEndpoint> for Peer {
    type Result = ResponseActFuture<Self, PeerResult<String>>;
    fn handle(
        &mut self,
        _msg: CreateSubscriptionEndpoint,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let rtc_core = self.rtc_core.clone();
        let endpoint_id = self.subscribe_endpoint.clone();
        Box::pin(
            async move {
                rtc_core
                    .send(CreateEndpointOffer { endpoint_id })
                    .await
                    .map_err(|error| PeerError::Rtc(error.to_string()))?
                    .map_err(PeerError::from)
            }
            .into_actor(self),
        )
    }
}

#[derive(Message)]
#[rtype(result = "PeerResult<String>")]
pub struct CompleteSubscriptionEndpoint {
    pub answer: String,
}

impl Handler<CompleteSubscriptionEndpoint> for Peer {
    type Result = ResponseActFuture<Self, PeerResult<String>>;

    fn handle(
        &mut self,
        msg: CompleteSubscriptionEndpoint,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let rtc_core = self.rtc_core.clone();
        let endpoint_id = self.subscribe_endpoint.clone();
        let succeeded_endpoint_id = endpoint_id.clone();
        Box::pin(
            async move {
                rtc_core
                    .send(ApplyEndpointAnswer {
                        endpoint_id,
                        answer: msg.answer,
                    })
                    .await
                    .map_err(|error| PeerError::Rtc(error.to_string()))?
                    .map_err(PeerError::from)?;
                Ok(String::new())
            }
            .into_actor(self)
            .map(move |result, actor, _ctx| {
                if result.is_ok() {
                    actor.parent_addr.do_send(SubscriptionEndpointSucceeded {
                        peer_id: actor.id.clone(),
                        endpoint_id: succeeded_endpoint_id.clone(),
                    });
                }
                result
            }),
        )
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct PeerShutdown;

impl Handler<PeerShutdown> for Peer {
    type Result = ResponseActFuture<Self, ()>;
    fn handle(&mut self, _msg: PeerShutdown, _ctx: &mut Self::Context) -> Self::Result {
        let endpoints = [
            self.publish_endpoint.clone(),
            self.subscribe_endpoint.clone(),
        ];
        let publish_endpoint = self.publish_endpoint.clone();
        let rtc_core = self.rtc_core.clone();
        let parent = self.parent_addr.clone();
        let peer_id = self.id.clone();
        Box::pin(
            async move {
                for endpoint_id in endpoints {
                    let _ = rtc_core
                        .send(CloseEndpoint {
                            endpoint_id,
                            reason: "peer shutdown".to_owned(),
                        })
                        .await;
                }
                parent.do_send(PeerStopped {
                    id: peer_id,
                    publish_endpoint,
                });
            }
            .into_actor(self)
            .map(|_, _, ctx| ctx.stop()),
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
        Self(user_uuid.into())
    }
    pub fn as_user_uuid(&self) -> String {
        self.0.clone()
    }
}

impl From<&str> for PeerId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}
