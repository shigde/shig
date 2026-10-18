use crate::sfu::peer::PeerId;
use crate::sfu::rtc::endpoint_event::IceGatheringEvents;
use crate::sfu::rtc::{RtcError, RtcResult};
use crate::sfu::rtc::port_allocator::PortLease;
use derive_more::Display;
use std::sync::Arc;
use tokio::sync::broadcast;
use webrtc::peer_connection::{PeerConnection, RTCIceGatheringState, RTCSessionDescription};

pub struct Endpoint {
    pub id: EndpointId,
    pub peer_connection: Arc<dyn PeerConnection>,
    ice_gathering_events: IceGatheringEvents,
    _port_lease: PortLease,
}

impl Endpoint {
    pub fn new(
        id: EndpointId,
        peer_connection: Arc<dyn PeerConnection>,
        port_lease: PortLease,
        ice_gathering_events: IceGatheringEvents,
    ) -> Self {
        Self {
            id,
            peer_connection,
            ice_gathering_events,
            _port_lease: port_lease,
        }
    }

    /// Produces a non-trickle SDP only after its ICE candidates are complete.
    pub async fn set_local_description_and_gather(
        &self,
        description: RTCSessionDescription,
    ) -> RtcResult<RTCSessionDescription> {
        // Subscribe before setting the description so a fast gathering completion cannot be lost.
        let mut gathering_events = self.ice_gathering_events.subscribe();

        self.peer_connection
            .set_local_description(description)
            .await
            .map_err(|err| RtcError::EndpointBuild(err.to_string()))?;

        loop {
            match gathering_events.recv().await {
                Ok(RTCIceGatheringState::Complete) => break,
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => {
                    return Err(RtcError::EndpointBuild(
                        "ICE gathering event stream closed".into(),
                    ));
                }
            }
        }

        self.peer_connection.local_description().await.ok_or_else(|| {
            RtcError::EndpointBuild("missing local description after ICE gathering".into())
        })
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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn as_string(&self) -> String {
        self.to_string()
    }

    pub(crate) fn channel_label(&self) -> &str {
        match self {
            EndpointKind::Publish => "whip",
            EndpointKind::Subscribe => "whep",
        }
    }
}
