use crate::sfu::rtc::endpoint::EndpointId;
use actix::{Message, Recipient};
use async_trait::async_trait;
use std::sync::Arc;
use webrtc::data_channel::DataChannel;
use webrtc::media_stream::track_remote::TrackRemote;
use webrtc::peer_connection::{
    PeerConnectionEventHandler, RTCIceConnectionState, RTCIceGatheringState,
    RTCPeerConnectionIceErrorEvent, RTCPeerConnectionIceEvent, RTCPeerConnectionState,
    RTCSignalingState,
};

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
