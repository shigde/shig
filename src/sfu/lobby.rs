use crate::relay::state::RelayState;
use crate::sfu::db::message::{AddParticipant, RemoveParticipant};
use crate::sfu::db::DbActor;
use crate::sfu::endpoint::EndpointId;
use crate::sfu::error::{LobbyError, LobbyResult};
use crate::sfu::peer::{
    CompleteSubscriptionEndpoint, CreatePublishEndpoint, CreateSubscriptionEndpoint, Peer, PeerId,
    PeerRole, PeerShutdown,
};
use crate::sfu::relay::actor::{
    AttachRelaySource, DetachRelaySource, RelayActor, RelayShutdown, StartRelayMediaStream,
    StopRelayMediaStream,
};
use crate::sfu::rtc::core_actor::RtcCoreActor;
use crate::sfu::rtc::media_command::{AssignLobby, ReleaseLobby};
use crate::sfu::rtc::pool_actor::RtcPoolActor;
use crate::sfu::{LobbyStopped, Sfu};
use crate::worker::manager::WorkerManager;
use actix::{
    Actor, ActorContext, ActorFutureExt, Addr, AsyncContext, Context, Handler, Message,
    ResponseActFuture, WrapFuture,
};
use moq_relay::AuthToken;
use std::collections::HashMap;

/// Identity of a lobby in the SFU domain.
#[derive(Debug, Clone, PartialEq, Eq, Hash, derive_more::Display)]
pub struct LobbyId(String);

impl LobbyId {
    pub fn new<S: Into<String>>(value: S) -> Self {
        Self(value.into())
    }
}

/// Owns the logical peers and maps their two RTC endpoints back to a participant.
pub struct Lobby {
    id: LobbyId,
    stream_uuid: String,
    #[allow(dead_code)]
    host_uuid: String,
    peers: HashMap<PeerId, Addr<Peer>>,
    parent_addr: Addr<Sfu>,
    db_actor_addr: Addr<DbActor>,
    rtc_pool: Addr<RtcPoolActor>,
    rtc_core: Option<Addr<RtcCoreActor>>,
    relay_addr: Addr<RelayActor>,
    shutting_down: bool,
}

impl Lobby {
    pub fn new(
        id: LobbyId,
        stream_uuid: String,
        host_uuid: String,
        parent_addr: Addr<Sfu>,
        db_actor_addr: Addr<DbActor>,
        rtc_pool: Addr<RtcPoolActor>,
        relay_state: RelayState,
        worker_manager: Addr<WorkerManager>,
    ) -> Self {
        let relay_addr = RelayActor::new(relay_state, worker_manager, stream_uuid.clone()).start();
        Self {
            id,
            stream_uuid,
            host_uuid,
            peers: HashMap::new(),
            parent_addr,
            db_actor_addr,
            rtc_pool,
            rtc_core: None,
            relay_addr,
            shutting_down: false,
        }
    }

    fn stop(&mut self, ctx: &mut Context<Self>) {
        self.rtc_pool.do_send(ReleaseLobby(self.id.clone()));
        self.relay_addr.do_send(RelayShutdown);
        self.parent_addr.do_send(LobbyStopped {
            id: self.id.to_string(),
        });
        ctx.stop();
    }
}

impl Actor for Lobby {
    type Context = Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        log::info!("lobby actor lobby_id={} is alive", self.id);
        let pool = self.rtc_pool.clone();
        let lobby_id = self.id.clone();
        ctx.wait(
            async move { pool.send(AssignLobby(lobby_id)).await }
                .into_actor(self)
                .map(|result, actor, ctx| match result {
                    Ok(Ok(assignment)) => {
                        log::info!(
                            "lobby_id={} assigned to RTC core {} at {}",
                            actor.id,
                            assignment.core_id,
                            assignment.media_addr
                        );
                        actor.rtc_core = Some(assignment.core);
                    }
                    Ok(Err(error)) => {
                        log::error!(
                            "RTC core assignment failed for lobby_id={}: {}",
                            actor.id,
                            error
                        );
                        ctx.stop();
                    }
                    Err(error) => {
                        log::error!(
                            "RTC pool mailbox failed for lobby_id={}: {}",
                            actor.id,
                            error
                        );
                        ctx.stop();
                    }
                }),
        );
    }
}

#[derive(Message)]
#[rtype(result = "LobbyResult<String>")]
pub struct Publish {
    pub user_uuid: String,
    pub offer: String,
    pub role: PeerRole,
}

impl Handler<Publish> for Lobby {
    type Result = ResponseActFuture<Self, LobbyResult<String>>;
    fn handle(&mut self, msg: Publish, ctx: &mut Self::Context) -> Self::Result {
        let peer_id = PeerId::new(&msg.user_uuid);
        if self.peers.contains_key(&peer_id) {
            return Box::pin(actix::fut::err(LobbyError::PeerAlreadyExists(peer_id)));
        }
        let publish_endpoint = EndpointId::publish(self.id.clone(), peer_id.clone());
        let subscribe_endpoint = EndpointId::subscribe(self.id.clone(), peer_id.clone());
        let Some(rtc_core) = self.rtc_core.clone() else {
            return Box::pin(actix::fut::err(LobbyError::RtcCoreUnavailable(
                "lobby has no RTC core assignment".to_owned(),
            )));
        };
        let peer = Peer::new(
            peer_id.clone(),
            ctx.address(),
            msg.role,
            publish_endpoint.clone(),
            subscribe_endpoint.clone(),
            rtc_core,
        )
        .start();
        self.peers.insert(peer_id.clone(), peer.clone());
        self.db_actor_addr.do_send(AddParticipant {
            lobby_uuid: self.id.to_string(),
            stream_uuid: self.stream_uuid.clone(),
            user_uuid: msg.user_uuid,
        });

        Box::pin(
            async move {
                peer.send(CreatePublishEndpoint { offer: msg.offer })
                    .await
                    .map_err(LobbyError::MailboxError)?
                    .map_err(LobbyError::PeerInternalError)
            }
            .into_actor(self),
        )
    }
}

#[derive(Message)]
#[rtype(result = "LobbyResult<String>")]
pub struct Subscribe {
    pub user_uuid: String,
    pub kind: SubscribeKind,
    pub answer: Option<String>,
}

#[derive(Debug, derive_more::Display)]
pub enum SubscribeKind {
    Offer,
    Answer,
}

impl Handler<Subscribe> for Lobby {
    type Result = ResponseActFuture<Self, LobbyResult<String>>;
    fn handle(&mut self, msg: Subscribe, _ctx: &mut Self::Context) -> Self::Result {
        let peer_id = PeerId::new(msg.user_uuid);
        let Some(peer) = self.peers.get(&peer_id).cloned() else {
            return Box::pin(actix::fut::err(LobbyError::PeerNotExists(peer_id)));
        };
        Box::pin(
            async move {
                let result = match msg.kind {
                    SubscribeKind::Offer => peer.send(CreateSubscriptionEndpoint {}).await,
                    SubscribeKind::Answer => {
                        let answer = msg.answer.ok_or_else(|| {
                            LobbyError::StreamingError("missing WHEP answer".to_owned())
                        })?;
                        peer.send(CompleteSubscriptionEndpoint { answer }).await
                    }
                };
                result
                    .map_err(LobbyError::MailboxError)?
                    .map_err(LobbyError::PeerInternalError)
            }
            .into_actor(self),
        )
    }
}

#[derive(Message)]
#[rtype(result = "LobbyResult<()>")]
pub struct LeavePeer {
    pub user_uuid: String,
}

impl Handler<LeavePeer> for Lobby {
    type Result = LobbyResult<()>;
    fn handle(&mut self, msg: LeavePeer, _ctx: &mut Self::Context) -> Self::Result {
        let peer_id = PeerId::new(msg.user_uuid);
        let peer = self
            .peers
            .get(&peer_id)
            .ok_or_else(|| LobbyError::PeerNotExists(peer_id.clone()))?;
        peer.do_send(PeerShutdown);
        Ok(())
    }
}

#[derive(Message)]
#[rtype(result = "LobbyResult<()>")]
pub struct TimeoutPeer {
    #[allow(dead_code)]
    pub user_uuid: String,
}

impl Handler<TimeoutPeer> for Lobby {
    type Result = LobbyResult<()>;
    fn handle(&mut self, _msg: TimeoutPeer, _ctx: &mut Self::Context) -> Self::Result {
        Ok(())
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct LobbyShutdown;

impl Handler<LobbyShutdown> for Lobby {
    type Result = ();
    fn handle(&mut self, _msg: LobbyShutdown, ctx: &mut Self::Context) {
        self.shutting_down = true;
        for peer in self.peers.values() {
            peer.do_send(PeerShutdown);
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
    pub publish_endpoint: EndpointId,
}

impl Handler<PeerStopped> for Lobby {
    type Result = ();
    fn handle(&mut self, msg: PeerStopped, ctx: &mut Self::Context) {
        self.peers.remove(&msg.id);
        self.relay_addr.do_send(DetachRelaySource {
            endpoint_id: msg.publish_endpoint,
        });
        self.db_actor_addr.do_send(RemoveParticipant {
            lobby_uuid: self.id.to_string(),
            stream_uuid: self.stream_uuid.clone(),
            user_uuid: msg.id.as_user_uuid(),
        });
        if self.peers.is_empty() {
            self.stop(ctx);
        }
    }
}

/// Emitted by a peer after its publish endpoint has completed negotiation.
#[derive(Message)]
#[rtype(result = "()")]
pub struct PublishEndpointSucceeded {
    pub peer_id: PeerId,
    pub endpoint_id: EndpointId,
}

impl Handler<PublishEndpointSucceeded> for Lobby {
    type Result = ();

    fn handle(&mut self, message: PublishEndpointSucceeded, _ctx: &mut Self::Context) {
        log::info!(
            "publish endpoint succeeded, lobby_id={}, peer_id={}",
            self.id,
            message.peer_id
        );
        self.relay_addr.do_send(AttachRelaySource {
            endpoint_id: message.endpoint_id,
        });
    }
}

/// Emitted by a peer after its subscribe endpoint has completed negotiation.
#[derive(Message)]
#[rtype(result = "()")]
pub struct SubscriptionEndpointSucceeded {
    pub peer_id: PeerId,
    pub endpoint_id: EndpointId,
}

impl Handler<SubscriptionEndpointSucceeded> for Lobby {
    type Result = ();

    fn handle(&mut self, message: SubscriptionEndpointSucceeded, _ctx: &mut Self::Context) {
        log::info!(
            "subscription endpoint succeeded, lobby_id={}, peer_id={}, endpoint={:?}",
            self.id,
            message.peer_id,
            message.endpoint_id
        );
    }
}

#[derive(Message)]
#[rtype(result = "LobbyResult<()>")]
pub struct PublishStream {
    pub publishing: bool,
    pub auth_token: Option<AuthToken>,
}

impl Handler<PublishStream> for Lobby {
    type Result = ResponseActFuture<Self, LobbyResult<()>>;
    fn handle(&mut self, msg: PublishStream, _ctx: &mut Self::Context) -> Self::Result {
        let relay = self.relay_addr.clone();
        Box::pin(
            async move {
                if msg.publishing {
                    let auth_token = msg.auth_token.ok_or_else(|| {
                        LobbyError::StreamingError("missing auth_token".to_owned())
                    })?;
                    relay
                        .send(StartRelayMediaStream { auth_token })
                        .await
                        .map_err(|error| LobbyError::StreamingError(error.to_string()))?
                        .map_err(LobbyError::StreamingError)
                } else {
                    relay
                        .send(StopRelayMediaStream)
                        .await
                        .map_err(|error| LobbyError::StreamingError(error.to_string()))?
                        .map_err(LobbyError::StreamingError)
                }
            }
            .into_actor(self),
        )
    }
}
