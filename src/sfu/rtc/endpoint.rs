use crate::sfu::peer::PeerId;
use crate::sfu::rtc::port_allocator::PortLease;
use actix::{Message, Recipient};
use async_trait::async_trait;
use derive_more::Display;
use std::sync::Arc;
use webrtc::data_channel::DataChannel;
use webrtc::media_stream::track_remote::TrackRemote;
use webrtc::peer_connection::{
    PeerConnection, PeerConnectionEventHandler, RTCIceConnectionState, RTCIceGatheringState,
    RTCPeerConnectionIceErrorEvent, RTCPeerConnectionIceEvent, RTCPeerConnectionState,
    RTCSignalingState,
};

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

#[derive(Message)]
#[rtype(result = "()")]
pub enum EndpointEvent {
    NegotiationNeeded {
        endpoint_id: EndpointId,
    },
    IceCandidate {
        endpoint_id: EndpointId,
        event: RTCPeerConnectionIceEvent,
    },
    IceCandidateError {
        endpoint_id: EndpointId,
        event: RTCPeerConnectionIceErrorEvent,
    },
    SignalingStateChange {
        endpoint_id: EndpointId,
        state: RTCSignalingState,
    },
    IceConnectionStateChange {
        endpoint_id: EndpointId,
        state: RTCIceConnectionState,
    },
    IceGatheringStateChange {
        endpoint_id: EndpointId,
        state: RTCIceGatheringState,
    },
    ConnectionStateChange {
        endpoint_id: EndpointId,
        state: RTCPeerConnectionState,
    },
    DataChannel {
        endpoint_id: EndpointId,
        data_channel: Arc<dyn DataChannel>,
    },
    Track {
        endpoint_id: EndpointId,
        track: Arc<dyn TrackRemote>,
    },
}

pub struct EndpointEventHandler {
    endpoint_id: EndpointId,
    event_sink: Recipient<EndpointEvent>,
}

impl EndpointEventHandler {
    pub fn new(endpoint_id: EndpointId, event_sink: Recipient<EndpointEvent>) -> Self {
        Self {
            endpoint_id,
            event_sink,
        }
    }
}

#[async_trait]
impl PeerConnectionEventHandler for EndpointEventHandler {
    async fn on_negotiation_needed(&self) {
        self.event_sink.do_send(EndpointEvent::NegotiationNeeded {
            endpoint_id: self.endpoint_id.clone(),
        });
    }

    async fn on_ice_candidate(&self, event: RTCPeerConnectionIceEvent) {
        self.event_sink.do_send(EndpointEvent::IceCandidate {
            endpoint_id: self.endpoint_id.clone(),
            event,
        });
    }

    async fn on_ice_candidate_error(&self, event: RTCPeerConnectionIceErrorEvent) {
        self.event_sink.do_send(EndpointEvent::IceCandidateError {
            endpoint_id: self.endpoint_id.clone(),
            event,
        });
    }

    async fn on_signaling_state_change(&self, state: RTCSignalingState) {
        self.event_sink
            .do_send(EndpointEvent::SignalingStateChange {
                endpoint_id: self.endpoint_id.clone(),
                state,
            });
    }

    async fn on_ice_connection_state_change(&self, state: RTCIceConnectionState) {
        self.event_sink
            .do_send(EndpointEvent::IceConnectionStateChange {
                endpoint_id: self.endpoint_id.clone(),
                state,
            });
    }

    async fn on_ice_gathering_state_change(&self, state: RTCIceGatheringState) {
        self.event_sink
            .do_send(EndpointEvent::IceGatheringStateChange {
                endpoint_id: self.endpoint_id.clone(),
                state,
            });
    }

    async fn on_connection_state_change(&self, state: RTCPeerConnectionState) {
        self.event_sink
            .do_send(EndpointEvent::ConnectionStateChange {
                endpoint_id: self.endpoint_id.clone(),
                state,
            });
    }

    async fn on_data_channel(&self, data_channel: Arc<dyn DataChannel>) {
        self.event_sink.do_send(EndpointEvent::DataChannel {
            endpoint_id: self.endpoint_id.clone(),
            data_channel,
        });
    }

    async fn on_track(&self, track: Arc<dyn TrackRemote>) {
        self.event_sink.do_send(EndpointEvent::Track {
            endpoint_id: self.endpoint_id.clone(),
            track,
        });
    }
}
