use crate::sfu::lobby::LobbyId;
use crate::sfu::peer::PeerId;
use std::sync::atomic::{AtomicU64, Ordering};

pub type RtcEndpointId = u64;

static NEXT_RTC_ENDPOINT_ID: AtomicU64 = AtomicU64::new(1);

fn next_rtc_endpoint_id() -> RtcEndpointId {
    NEXT_RTC_ENDPOINT_ID.fetch_add(1, Ordering::Relaxed)
}

/// The role of one WebRTC connection belonging to a peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EndpointKind {
    Publish,
    Subscribe,
}

impl std::fmt::Display for EndpointKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Publish => f.write_str("publish"),
            Self::Subscribe => f.write_str("subscribe"),
        }
    }
}

/// Stable domain identity of a WebRTC endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EndpointId {
    rtc_id: RtcEndpointId,
    lobby_id: LobbyId,
    peer_id: PeerId,
    kind: EndpointKind,
}

impl EndpointId {
    pub fn new(
        rtc_id: RtcEndpointId,
        lobby_id: LobbyId,
        peer_id: PeerId,
        kind: EndpointKind,
    ) -> Self {
        Self {
            rtc_id,
            lobby_id,
            peer_id,
            kind,
        }
    }

    pub fn publish(lobby_id: LobbyId, peer_id: PeerId) -> Self {
        Self::new(
            next_rtc_endpoint_id(),
            lobby_id,
            peer_id,
            EndpointKind::Publish,
        )
    }

    pub fn subscribe(lobby_id: LobbyId, peer_id: PeerId) -> Self {
        Self::new(
            next_rtc_endpoint_id(),
            lobby_id,
            peer_id,
            EndpointKind::Subscribe,
        )
    }

    pub fn rtc_id(&self) -> RtcEndpointId {
        self.rtc_id
    }

    pub fn lobby_id(&self) -> &LobbyId {
        &self.lobby_id
    }

    pub fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }

    pub fn kind(&self) -> EndpointKind {
        self.kind
    }
}

impl std::fmt::Display for EndpointId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}:{}",
            self.rtc_id, self.kind, self.lobby_id, self.peer_id
        )
    }
}
