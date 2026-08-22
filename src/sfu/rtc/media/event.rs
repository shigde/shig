use rtc::peer_connection::sdp::RTCSessionDescription;
use rtc::peer_connection::transport::RTCIceCandidateInit;

use crate::sfu::endpoint::EndpointId;
use super::lobby::RtcLobbyId;

pub type RequestId = u64;

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum SFUEvent {
    Ok {
        request_id: RequestId,
        rtc_lobby_id: Option<RtcLobbyId>,
        endpoint_id: Option<EndpointId>,
    },
    Err {
        request_id: RequestId,
        rtc_lobby_id: Option<RtcLobbyId>,
        endpoint_id: Option<EndpointId>,
        reason: String,
    },
    Join {
        request_id: RequestId,
        rtc_lobby_id: RtcLobbyId,
        endpoint_id: EndpointId,
    },
    SessionDescription {
        request_id: RequestId,
        rtc_lobby_id: RtcLobbyId,
        endpoint_id: EndpointId,
        sdp: RTCSessionDescription,
    },
    IceCandidate {
        request_id: RequestId,
        rtc_lobby_id: RtcLobbyId,
        endpoint_id: EndpointId,
        candidate: RTCIceCandidateInit,
    },
    Leave {
        request_id: RequestId,
        rtc_lobby_id: RtcLobbyId,
        endpoint_id: EndpointId,
        reason: String,
    },
}

impl SFUEvent {
    pub fn request_id(&self) -> RequestId {
        match self {
            SFUEvent::Ok { request_id, .. } => *request_id,
            SFUEvent::Err { request_id, .. } => *request_id,
            SFUEvent::Join { request_id, .. } => *request_id,
            SFUEvent::SessionDescription { request_id, .. } => *request_id,
            SFUEvent::IceCandidate { request_id, .. } => *request_id,
            SFUEvent::Leave { request_id, .. } => *request_id,
        }
    }
    pub fn rtc_lobby_id(&self) -> Option<RtcLobbyId> {
        match self {
            SFUEvent::Ok { rtc_lobby_id, .. } => *rtc_lobby_id,
            SFUEvent::Err { rtc_lobby_id, .. } => *rtc_lobby_id,
            SFUEvent::Join { rtc_lobby_id, .. } => Some(*rtc_lobby_id),
            SFUEvent::SessionDescription { rtc_lobby_id, .. } => Some(*rtc_lobby_id),
            SFUEvent::IceCandidate { rtc_lobby_id, .. } => Some(*rtc_lobby_id),
            SFUEvent::Leave { rtc_lobby_id, .. } => Some(*rtc_lobby_id),
        }
    }

    pub fn endpoint_id(&self) -> Option<&EndpointId> {
        match self {
            SFUEvent::Ok { endpoint_id, .. } => endpoint_id.as_ref(),
            SFUEvent::Err { endpoint_id, .. } => endpoint_id.as_ref(),
            SFUEvent::Join { endpoint_id, .. } => Some(endpoint_id),
            SFUEvent::SessionDescription { endpoint_id, .. } => Some(endpoint_id),
            SFUEvent::IceCandidate { endpoint_id, .. } => Some(endpoint_id),
            SFUEvent::Leave { endpoint_id, .. } => Some(endpoint_id),
        }
    }
}
