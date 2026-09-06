use crate::sfu::rtc::endpoint::{Endpoint, EndpointId};
use crate::sfu::rtc::error::{RtcError, RtcResult};
use std::sync::Arc;
use webrtc::peer_connection::{PeerConnection, RTCSessionDescription};

/// Actor-owned connection that receives media from a peer.
pub struct PublishEndpoint {
    endpoint: Endpoint,
}

impl PublishEndpoint {
    pub fn new(endpoint: Endpoint) -> Self {
        Self { endpoint }
    }

    pub fn id(&self) -> &EndpointId {
        &self.endpoint.id
    }

    pub async fn accept_offer(&self, sdp: String) -> RtcResult<String> {
        let offer = RTCSessionDescription::offer(sdp)
            .map_err(|err| RtcError::PublishEndpoint(err.to_string()))?;
        self.endpoint
            .peer_connection
            .set_remote_description(offer)
            .await
            .map_err(|err| RtcError::PublishEndpoint(err.to_string()))?;

        let answer = self
            .endpoint
            .peer_connection
            .create_answer(None)
            .await
            .map_err(|err| RtcError::PublishEndpoint(err.to_string()))?;

        self.endpoint
            .peer_connection
            .set_local_description(answer.clone())
            .await
            .map_err(|err| RtcError::PublishEndpoint(err.to_string()))?;

        Ok(answer.sdp )
    }
}
