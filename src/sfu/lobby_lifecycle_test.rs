use super::*;
use actix::{MessageResult, SyncArbiter};
use diesel::{
    r2d2::{ConnectionManager, Pool},
    PgConnection,
};

#[derive(Message)]
#[rtype(result = "(bool, bool, bool)")]
struct Inspect;

impl Handler<Inspect> for Lobby {
    type Result = MessageResult<Inspect>;
    fn handle(&mut self, _: Inspect, _: &mut Self::Context) -> Self::Result {
        MessageResult((
            self.host_present,
            self.host_timer.is_some(),
            self.shutting_down,
        ))
    }
}

fn disconnected<A: Actor<Context = Context<A>>>() -> Addr<A> {
    Context::<A>::new().address()
}

fn lobby(grace: u64) -> Addr<Lobby> {
    lobby_with_device(grace, None)
}

fn lobby_with_device(grace: u64, device: Option<PeerId>) -> Addr<Lobby> {
    // No database server is required: cleanup jobs fail fast against an unused port.
    let pool = Pool::builder()
        .min_idle(Some(0))
        .max_size(1)
        .connection_timeout(Duration::from_millis(1))
        .build_unchecked(ConnectionManager::<PgConnection>::new(
            "postgresql://127.0.0.1:1/test",
        ));
    let db = SyncArbiter::start(1, move || DbActor::new(pool.clone()));
    Lobby::create(move |ctx| {
        let config = SfuConfig {
            host_grace_seconds: grace,
            ..Default::default()
        };
        let mut peers = HashMap::new();
        // Keep an inert host slot, and a real guest to exercise actor teardown.
        peers.insert(PeerId::new("host"), disconnected::<Peer>());
        peers.insert(
            PeerId::new("guest"),
            Peer::new(
                PeerId::new("guest"),
                ctx.address(),
                PeerRole::Guest,
                config.clone(),
                PortAllocator::new(50000, 50001),
            )
            .start(),
        );
        if let Some(id) = device {
            peers.insert(id.clone(), Peer::new(
                id.clone(),
                ctx.address(),
                PeerRole::Device { name: "Camera 1".into() },
                config.clone(),
                PortAllocator::new(50000, 50001),
            ).start());
        }
        Lobby {
            id: "lobby".into(),
            stream_uuid: "stream".into(),
            host_uuid: "host".into(),
            sfu_config: config,
            peers: Box::new(peers),
            parent_addr: disconnected::<Sfu>(),
            db_actor_addr: db,
            relay_addr: disconnected::<RelayActor>(),
            port_allocator: PortAllocator::new(50000, 50001),
            router: Router::new(),
            shutting_down: false,
            streaming: false,
            host_present: false,
            host_leaving: false,
            host_timer: None,
        }
    })
}

#[derive(Message)]
#[rtype(result = "bool")]
struct HasPeer(PeerId);

impl Handler<HasPeer> for Lobby {
    type Result = bool;

    fn handle(&mut self, msg: HasPeer, _: &mut Self::Context) -> bool {
        self.peers.contains_key(&msg.0)
    }
}

#[actix_rt::test]
async fn only_the_host_can_kick_peers() {
    let id = PeerId::new("camera");
    let lobby = lobby_with_device(120, Some(id.clone()));
    presence(&lobby, true).await;
    let subscription = lobby.send(Subscribe {
        kind: SubscribeKind::Offer,
        user_uuid: "camera".into(),
        answer: None,
    }).await.unwrap();
    assert!(matches!(subscription, Err(LobbyError::PeerInternalError(PeerError::SubscribeNotAllowed))));
    lobby.send(KickPeer { host_uuid: "guest".into(), id: id.clone() }).await.unwrap();
    lobby.send(KickPeer { host_uuid: "guest".into(), id: PeerId::new("host") }).await.unwrap();
    assert!(lobby.send(HasPeer(id.clone())).await.unwrap());

    lobby.send(KickPeer { host_uuid: "host".into(), id: id.clone() }).await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while lobby.send(HasPeer(id.clone())).await.unwrap() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.expect("host kick should remove the camera peer");
    assert!(lobby.send(HasPeer(PeerId::new("guest"))).await.unwrap());
    let guest = PeerId::new("guest");
    lobby.send(KickPeer { host_uuid: "host".into(), id: guest.clone() }).await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while lobby.send(HasPeer(guest.clone())).await.unwrap() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.expect("host kick should also remove a guest peer");
}

async fn presence(lobby: &Addr<Lobby>, present: bool) {
    lobby
        .send(PeerPresence {
            id: PeerId::new("host"),
            present,
        })
        .await
        .unwrap();
}

#[test]
fn host_status_wire_format_and_default_grace_are_stable() {
    use crate::sfu::rtc::data_channel::DataChannelMsg;
    let message = DataChannelMsg::HostStatus(HostStatus::Waiting);
    assert_eq!(
        serde_json::to_value(message).unwrap(),
        serde_json::json!({"type": "4", "data": "waiting"})
    );
    let config: SfuConfig = toml::from_str("port_min = 50000").unwrap();
    assert_eq!(config.host_grace_seconds, 120);
    let config: SfuConfig = toml::from_str("host_grace_seconds = 180").unwrap();
    assert_eq!(config.host_grace_seconds, 180);
}

#[actix_rt::test]
async fn guests_are_rejected_until_host_connects_and_presence_is_stream_specific() {
    let lobby = lobby(120);
    let result = lobby
        .send(Publish {
            user_uuid: "new-guest".into(),
            stream_uuid: "stream".into(),
            offer: String::new(),
            role: PeerRole::Guest,
        })
        .await
        .unwrap();
    assert!(matches!(result, Err(LobbyError::HostAbsent)));
    let device = |stream_uuid: &str| Publish {
        user_uuid: "camera".into(),
        stream_uuid: stream_uuid.into(),
        offer: String::new(),
        role: PeerRole::Device { name: "Camera 1".into() },
    };
    assert!(matches!(
        lobby.send(device("stream")).await.unwrap(),
        Err(LobbyError::HostAbsent)
    ));
    presence(&lobby, true).await;
    assert!(matches!(
        lobby.send(device("other-stream")).await.unwrap(),
        Err(LobbyError::HostAbsent)
    ));
    assert!(lobby
        .send(HostPresence {
            stream_uuid: "stream".into()
        })
        .await
        .unwrap());
    assert!(!lobby
        .send(HostPresence {
            stream_uuid: "other".into()
        })
        .await
        .unwrap());
    let _ = lobby.send(LobbyShutdown {}).await;
    lobby.do_send(PeerStopped {
        id: PeerId::new("host"),
        role: PeerRole::Host,
    });
}

#[actix_rt::test]
async fn recovery_cancels_the_single_grace_timer() {
    let lobby = lobby(1);
    presence(&lobby, true).await;
    presence(&lobby, false).await;
    presence(&lobby, false).await;
    assert_eq!(lobby.send(Inspect).await.unwrap(), (false, true, false));
    presence(&lobby, true).await;
    assert_eq!(lobby.send(Inspect).await.unwrap(), (true, false, false));
    tokio::time::sleep(Duration::from_millis(1100)).await;
    assert_eq!(lobby.send(Inspect).await.unwrap(), (true, false, false));
    let _ = lobby.send(LobbyShutdown {}).await;
    lobby.do_send(PeerStopped {
        id: PeerId::new("host"),
        role: PeerRole::Host,
    });
}

#[actix_rt::test]
async fn absent_host_expires_and_shuts_down_remaining_guests() {
    let lobby = lobby(0);
    presence(&lobby, true).await;
    lobby
        .send(PeerStopped {
            id: PeerId::new("host"),
            role: PeerRole::Host,
        })
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while lobby.connected() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("grace expiry must stop guests and the lobby");
}

#[actix_rt::test]
async fn last_guest_leaving_closes_without_waiting_for_grace() {
    let lobby = lobby(120);
    presence(&lobby, true).await;
    lobby
        .send(PeerStopped {
            id: PeerId::new("host"),
            role: PeerRole::Host,
        })
        .await
        .unwrap();
    lobby
        .send(LeavePeer {
            user_uuid: "guest".into(),
        })
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while lobby.connected() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("empty lobby must stop immediately");
}

#[actix_rt::test]
async fn explicit_host_leave_ignores_late_connected_events() {
    let camera = PeerId::new("camera");
    let lobby = lobby_with_device(120, Some(camera.clone()));
    presence(&lobby, true).await;
    lobby
        .send(LeavePeer {
            user_uuid: "host".into(),
        })
        .await
        .unwrap()
        .unwrap();
    presence(&lobby, true).await;
    assert_eq!(lobby.send(Inspect).await.unwrap(), (false, true, false));
    tokio::time::timeout(Duration::from_secs(2), async {
        while lobby.send(HasPeer(camera.clone())).await.unwrap() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.expect("camera peer should close when the host leaves");
    assert!(lobby.send(HasPeer(PeerId::new("guest"))).await.unwrap());
    let _ = lobby.send(LobbyShutdown {}).await;
    lobby.do_send(PeerStopped {
        id: PeerId::new("host"),
        role: PeerRole::Host,
    });
}
