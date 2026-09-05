use crate::sfu::rtc::endpoint::{Endpoint, EndpointId};
use std::sync::Arc;
use webrtc::peer_connection::PeerConnection;

/// Actor-owned connection that sends subscribed media to a peer.
pub struct SubscribeEndpoint {
    endpoint: Endpoint,
}

impl SubscribeEndpoint {
    pub fn new(endpoint: Endpoint) -> Self {
        Self { endpoint }
    }

    pub fn id(&self) -> &EndpointId {
        &self.endpoint.id
    }

    pub fn peer_connection(&self) -> Arc<dyn PeerConnection> {
        Arc::clone(&self.endpoint.peer_connection)
    }
}
