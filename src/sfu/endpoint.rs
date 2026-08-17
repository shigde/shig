use crate::sfu::lobby::LobbyId;
use crate::sfu::peer::PeerId;

/// The role of one WebRTC connection belonging to a peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EndpointKind {
    Publish,
    Subscribe,
}

/// Stable domain identity of a WebRTC endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EndpointId {
    lobby_id: LobbyId,
    peer_id: PeerId,
    kind: EndpointKind,
}

impl EndpointId {
    pub fn publish(lobby_id: LobbyId, peer_id: PeerId) -> Self {
        Self {
            lobby_id,
            peer_id,
            kind: EndpointKind::Publish,
        }
    }

    pub fn subscribe(lobby_id: LobbyId, peer_id: PeerId) -> Self {
        Self {
            lobby_id,
            peer_id,
            kind: EndpointKind::Subscribe,
        }
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
