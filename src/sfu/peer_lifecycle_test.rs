use super::*;
use actix::MessageResult;

#[derive(Message)]
#[rtype(result = "[bool; 2]")]
struct DisconnectTimers;

impl Handler<DisconnectTimers> for Peer {
    type Result = MessageResult<DisconnectTimers>;

    fn handle(&mut self, _: DisconnectTimers, _: &mut Self::Context) -> Self::Result {
        MessageResult(self.disconnect_timers.map(|timer| timer.is_some()))
    }
}

fn peer(parent: Addr<Lobby>) -> Addr<Peer> {
    Peer::new(
        PeerId::new("peer"),
        parent,
        PeerRole::Guest,
        SfuConfig::default(),
        PortAllocator::new(50000, 50001),
    )
    .start()
}

fn state(kind: EndpointKind, state: RTCPeerConnectionState) -> EndpointEvent {
    EndpointEvent::ConnectionStateChange {
        endpoint_id: EndpointId::new(PeerId::new("peer"), kind),
        state,
    }
}

#[actix_rt::test]
async fn transient_disconnect_is_cancelled_only_by_recovery_of_the_same_endpoint() {
    let parent_context = Context::<Lobby>::new();
    let peer = peer(parent_context.address());
    peer.send(state(
        EndpointKind::Publish,
        RTCPeerConnectionState::Disconnected,
    ))
    .await
    .unwrap();
    peer.send(state(
        EndpointKind::Publish,
        RTCPeerConnectionState::Disconnected,
    ))
    .await
    .unwrap();
    peer.send(state(
        EndpointKind::Subscribe,
        RTCPeerConnectionState::Disconnected,
    ))
    .await
    .unwrap();
    assert_eq!(peer.send(DisconnectTimers).await.unwrap(), [true, true]);
    peer.send(state(
        EndpointKind::Subscribe,
        RTCPeerConnectionState::Connected,
    ))
    .await
    .unwrap();
    assert_eq!(peer.send(DisconnectTimers).await.unwrap(), [true, false]);
    peer.send(state(
        EndpointKind::Publish,
        RTCPeerConnectionState::Connected,
    ))
    .await
    .unwrap();
    assert_eq!(peer.send(DisconnectTimers).await.unwrap(), [false, false]);
    let _ = peer.send(PeerShutdown {}).await;
}

#[actix_rt::test]
async fn terminal_connection_states_stop_the_peer() {
    let parent_context = Context::<Lobby>::new();
    for kind in [EndpointKind::Publish, EndpointKind::Subscribe] {
        for terminal in [
            RTCPeerConnectionState::Failed,
            RTCPeerConnectionState::Closed,
        ] {
            let peer = peer(parent_context.address());
            peer.send(state(kind, terminal)).await.unwrap();
            tokio::time::timeout(Duration::from_secs(1), async {
                while peer.connected() {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .expect("terminal connection state must stop the peer actor");
        }
    }
}

#[actix_rt::test]
async fn persistent_disconnect_stops_the_peer_after_the_grace_period() {
    let parent_context = Context::<Lobby>::new();
    let peer = peer(parent_context.address());
    peer.send(state(
        EndpointKind::Publish,
        RTCPeerConnectionState::Disconnected,
    ))
    .await
    .unwrap();
    assert!(peer.connected());
    tokio::time::timeout(Duration::from_secs(22), async {
        while peer.connected() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("persistent disconnect must stop the peer actor after 20 seconds");
}
