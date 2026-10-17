use crate::metrics;
use crate::relay::state::RelayState;
use crate::sfu::config::SfuConfig;
use crate::sfu::db::message::{AddParticipant, RemoveParticipant, SetLobbyOnline};
use crate::sfu::db::DbActor;
use crate::sfu::error::{LobbyError, LobbyResult};
use crate::sfu::media::router::Router;
use crate::sfu::media::{
    ActivateMedia, AddMedia, MediaId, MuteMedia, MuteRemoteMedia, RemoveMedia,
};
use crate::sfu::peer::{
    Peer, PeerCreatePublishAnswer, PeerCreateSubscribeOffer, PeerId, PeerReceiveSubscribeAnswer,
    PeerRole, PeerShutdown,
};
use crate::sfu::relay::actor::RelayActor;
use crate::sfu::relay::message::{StartRelayMediaStream, StopRelayMediaStream};
use crate::sfu::rtc::PortAllocator;
use crate::sfu::{LobbyStopped, Sfu};
use actix::{
    Actor, ActorContext, ActorFutureExt, Addr, AsyncContext, AtomicResponse, Context, Handler, Message,
    ResponseActFuture, WrapFuture,
};
use derive_more::Display;
use moq_relay::auth::Token as AuthToken;
use std::collections::HashMap;

pub struct Lobby {
    id: String,
    stream_uuid: String,
    #[allow(dead_code)]
    host_uuid: String, // owner of this stream
    sfu_config: SfuConfig,
    peers: Box<HashMap<PeerId, Addr<Peer>>>,
    parent_addr: Addr<Sfu>,
    db_actor_addr: Addr<DbActor>,
    relay_addr: Addr<RelayActor>,
    port_allocator: PortAllocator,
    router: Router,
    shutting_down: bool,
    streaming: bool,
}

impl Lobby {
    pub fn new(
        uuid: String,
        stream_uuid: String,
        host_uuid: String,
        sfu_config: SfuConfig,
        parent_addr: Addr<Sfu>,
        db_actor_addr: Addr<DbActor>,
        relay_state: RelayState,
        port_allocator: PortAllocator,
    ) -> Self {
        let relay_addr =
            RelayActor::new(relay_state, stream_uuid.clone()).start();

        Self {
            id: uuid,
            stream_uuid,
            host_uuid,
            sfu_config,
            peers: Box::new(HashMap::new()),
            parent_addr,
            db_actor_addr,
            relay_addr,
            port_allocator,
            router: Router::new(),
            shutting_down: false,
            streaming: false,
        }
    }

    fn stop(&mut self, ctx: &mut Context<Self>) {
        if self.streaming {
            log::info!(
                "stopping active relay media stream before lobby stop, lobby_id={}, stream_uuid={}",
                self.id,
                self.stream_uuid
            );
            self.relay_addr.do_send(StopRelayMediaStream {});
            self.streaming = false;
        }

        ctx.stop();
    }

    fn remove_media(&mut self, media_id: MediaId) {
        if let Some(media) = self.router.medias.remove(&media_id) {
            for (peer_id, peer_addr) in self.peers.iter() {
                if peer_id != &media.peer_id {
                    peer_addr.do_send(RemoveMedia {
                        media_id: media_id.clone(),
                    });
                }
            }
        }
    }
}

impl Actor for Lobby {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        log::info!("Lobby actor lobby_id={} is alive", self.id);
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        log::info!("Lobby actor lobby_id={} is stopped", self.id);
        self.parent_addr.do_send(LobbyStopped {
            id: self.id.clone(),
        });
    }
}

#[derive(Message)]
#[rtype(result = " LobbyResult<String>")]
pub struct Publish {
    pub user_uuid: String,
    pub offer: String,
    pub role: PeerRole,
}

impl Handler<Publish> for Lobby {
    type Result = ResponseActFuture<Self, LobbyResult<String>>;

    fn handle(&mut self, msg: Publish, ctx: &mut Self::Context) -> Self::Result {
        let peer_id = PeerId::new(msg.user_uuid.clone());
        let user_uuid = msg.user_uuid.clone();
        let lobby_uuid = self.id.clone();
        let stream_uuid = self.stream_uuid.clone();

        // If the peer already exists, directly return a completed Future with error
        if self.peers.contains_key(&peer_id) {
            return Box::pin(
                async move { Err(LobbyError::PeerAlreadyExists(peer_id)) }.into_actor(self),
            );
        }

        let peer_addr = Peer::new(
            peer_id.clone(),
            ctx.address(),
            msg.role,
            self.sfu_config.clone(),
            self.port_allocator.clone(),
        )
        .start();
        self.peers.insert(peer_id.clone(), peer_addr.clone());
        metrics::set_active_lobby_peers(&self.id, self.peers.len());

        let db_actor = self.db_actor_addr.clone();
        let offer = msg.offer.clone();
        let failed_peer_addr = peer_addr.clone();
        let fut = async move {
            // The next WHEP/leave request reads these records directly from the database.
            db_actor
                .send(SetLobbyOnline {
                    lobby_uuid: lobby_uuid.clone(),
                    stream_uuid: stream_uuid.clone(),
                })
                .await
                .map_err(LobbyError::MailboxError)?
                .map_err(LobbyError::PersistenceError)?;
            db_actor
                .send(AddParticipant {
                    lobby_uuid,
                    stream_uuid,
                    user_uuid,
                })
                .await
                .map_err(LobbyError::MailboxError)?
                .map_err(LobbyError::PersistenceError)?;
            let result = peer_addr.send(PeerCreatePublishAnswer { offer }).await;

            match result {
                Ok(val) => match val {
                    Ok(answer) => Ok(answer),
                    Err(e) => Err(LobbyError::PeerInternalError(e)),
                },
                Err(e) => Err(LobbyError::MailboxError(e)),
            }
        }
        .into_actor(self);

        Box::pin(fut.map(move |result, _actor, _ctx| {
            if let Err(ref error) = result {
                log::warn!(
                    "publish setup failed; shutting down peer, peer_id={}, error={}",
                    peer_id,
                    error
                );
                failed_peer_addr.do_send(PeerShutdown {});
            }
            result
        }))
    }
}

#[derive(Message)]
#[rtype(result = " LobbyResult<String>")]
pub struct Subscribe {
    pub kind: SubscribeKind,
    pub user_uuid: String,
    pub answer: Option<String>,
}

#[derive(Display)]
pub enum SubscribeKind {
    Offer,
    Answer,
}

impl Handler<Subscribe> for Lobby {
    type Result = ResponseActFuture<Self, LobbyResult<String>>;

    fn handle(&mut self, msg: Subscribe, _ctx: &mut Self::Context) -> Self::Result {
        let peer_id = PeerId::new(msg.user_uuid.clone());

        let peer_addr = match self.peers.get(&peer_id) {
            Some(addr) => addr.clone(),
            None => {
                return Box::pin(
                    async move { Err(LobbyError::PeerNotExists(peer_id)) }.into_actor(self),
                );
            }
        };

        let answer_option = msg.answer.clone();
        let kind = msg.kind;
        let failed_peer_addr = peer_addr.clone();
        let medias = match kind {
            SubscribeKind::Offer => self.router.get_medias_without_peer(&peer_id),
            SubscribeKind::Answer => vec![],
        };

        log::info!(
            "subscribing Peer peer_id={} has medias medias_len={}",
            peer_id,
            medias.len()
        );

        let fut = async move {
            let result = match kind {
                SubscribeKind::Offer => peer_addr.send(PeerCreateSubscribeOffer { medias }).await,
                SubscribeKind::Answer => {
                    let answer = answer_option.unwrap();
                    peer_addr.send(PeerReceiveSubscribeAnswer { answer }).await
                }
            };

            match result {
                Ok(val) => match val {
                    Ok(answer) => Ok(answer),
                    Err(e) => Err(LobbyError::PeerInternalError(e)),
                },
                Err(e) => Err(LobbyError::MailboxError(e)),
            }
        }
        .into_actor(self);

        Box::pin(fut.map(move |result, _actor, _ctx| {
            if let Err(ref error) = result {
                log::warn!(
                    "subscribe setup failed; shutting down peer, peer_id={}, error={}",
                    peer_id,
                    error
                );
                failed_peer_addr.do_send(PeerShutdown {});
            }
            result
        }))
    }
}

#[derive(Message)]
#[rtype(result = " LobbyResult<()>")]
pub struct LeavePeer {
    pub user_uuid: String,
}

impl Handler<LeavePeer> for Lobby {
    type Result = LobbyResult<()>;

    fn handle(&mut self, msg: LeavePeer, _: &mut Self::Context) -> Self::Result {
        let peer_id = PeerId::new(msg.user_uuid.clone());

        let Some(peer_add) = self.peers.get(&peer_id).cloned() else {
            return Err(LobbyError::PeerNotExists(peer_id));
        };

        // remove all medias
        let medias = self.router.get_medias_of_peer(&peer_id);
        for media in medias {
            self.remove_media(media.id);
        }

        log::info!("send shutdown because peer_id={} is leaving lobby", peer_id);
        peer_add.do_send(PeerShutdown {});
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
    type Result = AtomicResponse<Self, ()>;

    fn handle(&mut self, _msg: LobbyShutdown, _ctx: &mut Self::Context) -> Self::Result {
        self.shutting_down = true;

        for (_, addr) in self.peers.iter() {
            addr.do_send(PeerShutdown {});
        }

        let relay = self.relay_addr.clone();
        // The SFU waits for this reply before terminating its runtime. Pause peer-stop
        // handling until the relay has joined its GStreamer teardown.
        AtomicResponse::new(Box::pin(
            async move { let _ = relay.send(StopRelayMediaStream {}).await; }
                .into_actor(self)
                .map(|(), actor, ctx| {
                    actor.streaming = false;
                    if actor.peers.is_empty() { actor.stop(ctx); }
                }),
        ))
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
        if self.peers.remove(&msg.id).is_none() {
            return;
        }
        let medias = self.router.get_medias_of_peer(&msg.id);
        for media in medias {
            self.remove_media(media.id);
        }
        self.router.remove_targets_for_peer(&msg.id);
        if msg.id.as_str() == self.host_uuid && self.streaming {
            self.streaming = false;
            self.relay_addr.do_send(StopRelayMediaStream {});
        }
        metrics::set_active_lobby_peers(&self.id, self.peers.len());
        self.db_actor_addr.do_send(RemoveParticipant {
            lobby_uuid: self.id.clone(),
            stream_uuid: self.stream_uuid.clone(),
            user_uuid: msg.id.as_user_uuid(),
        });

        if self.peers.is_empty() {
            self.stop(ctx);
        }
    }
}

impl Handler<ActivateMedia> for Lobby {
    type Result = ();

    fn handle(&mut self, msg: ActivateMedia, _ctx: &mut Self::Context) -> Self::Result {
        let media_id = msg.media_id;
        let Some(_) = self.router.activate_target(&media_id, msg.target) else {
            log::debug!("cannot activate missing media route, media_id={}", media_id);
            return;
        };

        log::debug!("activated media route, media_id={}", media_id);
    }
}

impl Handler<AddMedia> for Lobby {
    type Result = ();

    fn handle(&mut self, msg: AddMedia, _ctx: &mut Self::Context) -> Self::Result {
        log::info!(
            "Handle add Media: peer_id={}, media_id={}, kind={}",
            msg.media.peer_id,
            msg.media.id,
            msg.media.kind
        );
        if self.router.medias.contains_key(&msg.media.id) {
            log::warn!(
                "media already exists, peer_id={}, media_id={}, kind={}",
                msg.media.peer_id,
                msg.media.id,
                msg.media.kind
            );
            return;
        }

        match self
            .router
            .medias
            .insert(msg.media.id.clone(), msg.media.clone())
        {
            Some(_) => log::warn!(
                "media already exists, peer_id={}, media_id={}, kind={}",
                msg.media.peer_id,
                msg.media.id,
                msg.media.kind
            ),
            None => {
                log::info!(
                    "add media, peer_id={}, media_id={}, kind={}",
                    msg.media.peer_id,
                    msg.media.id,
                    msg.media.kind
                );
                for (peer_id, peer_addr) in self.peers.iter() {
                    if peer_id != &msg.media.peer_id {
                        peer_addr.do_send(AddMedia {
                            media: msg.media.clone(),
                        });
                    }
                }
            }
        }
    }
}

impl Handler<RemoveMedia> for Lobby {
    type Result = ();

    fn handle(&mut self, msg: RemoveMedia, _ctx: &mut Self::Context) -> Self::Result {
        self.remove_media(msg.media_id);
    }
}

impl Handler<MuteMedia> for Lobby {
    type Result = ();

    fn handle(&mut self, msg: MuteMedia, _ctx: &mut Self::Context) -> Self::Result {
        let Some(media) = self
            .router
            .get_media_of_peer_by_mid(&msg.peer_id, msg.mid.as_str())
        else {
            return;
        };
        media.set_mut(msg.mute);

        for (peer_id, peer_addr) in self.peers.iter() {
            if peer_id != &media.peer_id {
                peer_addr.do_send(MuteRemoteMedia {
                    media_id: media.id.clone(),
                    mute: msg.mute,
                });
            }
        }
    }
}

#[derive(Message)]
#[rtype(result = " LobbyResult<()>")]
pub struct PublishStream {
    pub publishing: bool,
    pub auth_token: Option<AuthToken>,
}

impl Handler<PublishStream> for Lobby {
    type Result = ResponseActFuture<Self, LobbyResult<()>>;

    fn handle(&mut self, msg: PublishStream, _ctx: &mut Self::Context) -> Self::Result {
        let relay_addr = self.relay_addr.clone();
        if msg.publishing {
            let Some(auth_token) = msg.auth_token else {
                return Box::pin(
                    async { Err(LobbyError::StreamingError("missing auth_token".to_string())) }
                        .into_actor(self),
                );
            };

            let media_stream = self.router.get_media_stream(&PeerId::new(&self.host_uuid));

            if media_stream.audio.is_none() && media_stream.video.is_none() {
                return Box::pin(
                    async {
                        Err(LobbyError::StreamingError(
                            "no audio and video stream exists".to_string(),
                        ))
                    }
                    .into_actor(self),
                );
            }

            let stream_start = StartRelayMediaStream {
                media_stream,
                auth_token,
            };

            let stream_id = self.stream_uuid.clone();
            Box::pin(
                async move {
                    log::info!("start relay media stream: stream_uuid={:?} ", stream_id);
                    relay_addr
                        .send(stream_start)
                        .await
                        .map_err(|e| {
                            LobbyError::StreamingError(format!(
                                "relay mailbox, stream_id={:?},error: {}",
                                stream_id, e
                            ))
                        })?
                        .map_err(|e| {
                            LobbyError::StreamingError(format!(
                                "relay start, stream_id={:?}, error: {}",
                                stream_id, e
                            ))
                        })?;

                    Ok(())
                }
                .into_actor(self)
                .map(|result, actor, _ctx| {
                    if result.is_ok() {
                        actor.streaming = true;
                    }
                    result
                }),
            )
        } else {
            let stream_id = self.stream_uuid.clone();
            Box::pin(
                async move {
                    log::info!("stop relay media stream: stream_uuid={:?} ", stream_id);
                    relay_addr
                        .send(StopRelayMediaStream {})
                        .await
                        .map_err(|e| {
                            LobbyError::StreamingError(format!(
                                "relay mailbox, stream_id={:?}, error: {}",
                                stream_id, e
                            ))
                        })?
                        .map_err(|e| {
                            LobbyError::StreamingError(format!(
                                "relay stop,  stream_id={:?}, error: {}",
                                stream_id, e
                            ))
                        })?;

                    Ok(())
                }
                .into_actor(self)
                .map(|result, actor, _ctx| {
                    if result.is_ok() {
                        actor.streaming = false;
                    }
                    result
                }),
            )
        }
    }
}
