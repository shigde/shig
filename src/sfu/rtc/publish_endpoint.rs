use crate::sfu::rtc::endpoint::{Endpoint, EndpointId};
use crate::sfu::rtc::{RtcError, RtcResult};
use webrtc::peer_connection::RTCSessionDescription;

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

        Ok(answer.sdp)
    }

    pub async fn shutdown(&self) -> RtcResult<()> {
        self.endpoint
            .peer_connection
            .close()
            .await
            .map_err(|err| RtcError::PublishEndpoint(err.to_string()))
    }
}
