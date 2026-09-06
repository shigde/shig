use crate::sfu::peer::PeerId;
use crate::sfu::rtc::port_allocator::PortLease;
use derive_more::Display;
use std::sync::Arc;
use webrtc::peer_connection::PeerConnection;

pub struct Endpoint {
    pub id: EndpointId,
    pub peer_connection: Arc<dyn PeerConnection>,
    _port_lease: PortLease,
}

impl Endpoint {
    pub fn new(
        id: EndpointId,
        peer_connection: Arc<dyn PeerConnection>,
        port_lease: PortLease,
    ) -> Self {
        Self {
            id,
            peer_connection,
            _port_lease: port_lease,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Display)]
#[display(fmt = "{}:{}", peer_id, kind)]
pub struct EndpointId {
    peer_id: PeerId,
    kind: EndpointKind,
}

impl EndpointId {
    pub fn new<P: Into<PeerId>, K: Into<EndpointKind>>(peer_id: P, kind: K) -> Self {
        Self {
            peer_id: peer_id.into(),
            kind: kind.into(),
        }
    }

    pub fn as_string(&self) -> String {
        self.to_string()
    }

    pub fn as_peer_id(&self) -> PeerId {
        self.peer_id.clone()
    }

    pub fn as_kind(&self) -> EndpointKind {
        self.kind
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display)]
pub enum EndpointKind {
    #[display(fmt = "publish")]
    Publish,
    #[display(fmt = "subscribe")]
    Subscribe,
}

impl EndpointKind {
    pub fn is_publish(&self) -> bool {
        matches!(self, EndpointKind::Publish)
    }
    pub fn is_subscribe(&self) -> bool {
        matches!(self, EndpointKind::Subscribe)
    }

    pub fn as_string(&self) -> String {
        self.to_string()
    }
}
