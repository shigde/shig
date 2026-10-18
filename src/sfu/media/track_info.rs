use crate::sfu::media::{MediaPurpose, RtpCodecType};
use std::collections::HashMap;
use std::sync::Arc;
use webrtc::rtp_transceiver::RtpTransceiver;

#[derive(Debug, Clone)]
pub struct InboundTrackInfo {
    pub mid: String,
    pub kind: RtpCodecType,
    pub source_stream_id: String,
    pub source_track_id: String,
    /// The extmap IDs announced by the publishing peer for this media section.
    pub header_extensions: HashMap<u8, String>,
    pub purpose: MediaPurpose,
    pub muted: bool,
    pub info: TrackInfo,
}

impl InboundTrackInfo {
    pub fn new(kind: RtpCodecType) -> Self {
        Self {
            mid: "".to_string(),
            kind,
            source_stream_id: "".to_string(),
            source_track_id: "".to_string(),
            header_extensions: HashMap::new(),
            purpose: MediaPurpose::PARTICIPANT,
            muted: true,
            info: "".to_string(),
        }
    }
}

#[derive(Clone)]
pub struct OutboundTrackInfo {
    pub msid: String,
    pub purpose: MediaPurpose,
    pub muted: bool,
    pub info: TrackInfo,
    pub transceiver: Arc<dyn RtpTransceiver>,
}

impl OutboundTrackInfo {
    pub(crate) fn new(
        msid: String,
        tc: Arc<dyn RtpTransceiver>,
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
