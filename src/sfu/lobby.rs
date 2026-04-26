use crate::sfu::db::message::{AddParticipant, RemoveParticipant};
use crate::sfu::db::DbActor;
use crate::sfu::error::{LobbyError, LobbyResult};
use crate::sfu::media::router::Router;
use crate::sfu::media::{AddMedia, MediaId, MuteMedia, MuteRemoteMedia, RemoveMedia};
use crate::sfu::peer::{
    Peer, PeerId, PeerRole, PeerSending, PeerShutdown, PeerStartReceiving, PeerStartSending,
};
use crate::sfu::{LobbyStopped, Sfu};
use actix::{
    Actor, ActorContext, Addr, AsyncContext, Context, Handler, Message, ResponseActFuture,
    WrapFuture,
};
use derive_more::Display;
use std::collections::HashMap;

pub struct Lobby {
    id: String,
    stream_uuid: String,
    #[allow(dead_code)]
    host_uuid: String, // owner of this stream
    peers: Box<HashMap<PeerId, Addr<Peer>>>,
    parent_addr: Addr<Sfu>,
    db_actor_addr: Addr<DbActor>,
    #[allow(dead_code)]
    router: Router,
    shutting_down: bool,
}

impl Lobby {
    pub fn new(
        uuid: String,
        stream_uuid: String,
        host_uuid: String,
        parent_addr: Addr<Sfu>,
        db_actor_addr: Addr<DbActor>,
    ) -> Self {
        Self {
            id: uuid,
            stream_uuid,
            host_uuid,
            peers: Box::new(HashMap::new()),
            parent_addr,
            db_actor_addr,
            router: Router::new(),
            shutting_down: false,
        }
    }

    fn stop(&mut self, ctx: &mut Context<Self>) {
        self.parent_addr.do_send(LobbyStopped {
            id: self.id.clone(),
        });
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

        let peer_addr = Peer::new(peer_id.clone(), ctx.address(), msg.role).start();
        self.peers.insert(peer_id, peer_addr.clone());

        self.db_actor_addr.do_send(AddParticipant {
            lobby_uuid,
            stream_uuid,
            user_uuid,
        });

        let offer = msg.offer.clone();
        let fut = async move {
            let result = peer_addr.send(PeerStartReceiving { offer }).await;

            match result {
                Ok(val) => match val {
                    Ok(answer) => Ok(answer),
                    Err(e) => Err(LobbyError::PeerInternalError(e)),
                },
                Err(e) => Err(LobbyError::MailboxError(e)),
            }
        }
        .into_actor(self);

        Box::pin(fut)
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
        let medias = match kind {
            SubscribeKind::Offer => self.router.get_medias_without_peer(&peer_id),
            SubscribeKind::Answer => vec![],
        };

        log::info!("subscribing Peer peer_id={} has medias medias_len={}", peer_id, medias.len());

        let fut = async move {
            let result = match kind {
                SubscribeKind::Offer => peer_addr.send(PeerStartSending { medias }).await,
                SubscribeKind::Answer => {
                    let answer = answer_option.unwrap();
                    peer_addr.send(PeerSending { answer }).await
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

        Box::pin(fut)
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
    type Result = ();

    fn handle(&mut self, _msg: LobbyShutdown, ctx: &mut Self::Context) -> Self::Result {
        self.shutting_down = true;

        for (_, addr) in self.peers.iter() {
            addr.do_send(PeerShutdown {});
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
}

impl Handler<PeerStopped> for Lobby {
    type Result = ();

    fn handle(&mut self, msg: PeerStopped, ctx: &mut Self::Context) -> Self::Result {
        self.peers.remove(&msg.id);
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
}

impl Handler<PublishStream> for Lobby {
    type Result = LobbyResult<()>;

    fn handle(&mut self, msg: PublishStream, _ctx: &mut Self::Context) -> Self::Result {
        Ok(())
    }
}
