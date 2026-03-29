use crate::sfu::media::MediaPurpose;
use std::sync::Arc;
use webrtc::rtp_transceiver::rtp_codec::RTPCodecType;
use webrtc::rtp_transceiver::RTCRtpTransceiver;

#[derive(Debug, Clone)]
pub struct InboundTrackInfo {
    pub mid: String,
    pub kind: RTPCodecType,
    pub purpose: MediaPurpose,
    pub muted: bool,
    pub info: TrackInfo,
}

impl InboundTrackInfo {
    pub fn new(kind: RTPCodecType) -> Self {
        Self {
            mid: "".to_string(),
            kind,
            purpose: MediaPurpose::PARTICIPANT,
            muted: true,
            info: "".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OutboundTrackInfo {
    pub msid: String,
    pub purpose: MediaPurpose,
    pub muted: bool,
    pub info: TrackInfo,
    pub transceiver: Arc<RTCRtpTransceiver>,
}

impl OutboundTrackInfo {
    pub(crate) fn new(
        msid: String,
        tc: Arc<RTCRtpTransceiver>,
        purpose: MediaPurpose,
        muted: bool,
        info: TrackInfo,
    ) -> OutboundTrackInfo {
        OutboundTrackInfo {
            msid,
            purpose,
            muted,
            info,
            transceiver: tc,
        }
    }
}


pub type TrackInfo = String;
