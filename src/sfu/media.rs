use crate::sfu::peer::PeerId;
use actix::Message;
use derive_more::Display;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use webrtc::rtp::packet::Packet;
use webrtc::rtp_transceiver::rtp_codec::{RTCRtpCodecCapability, RTPCodecType};
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_local::{TrackLocal, TrackLocalWriter};

pub mod connector;
pub mod data_channel;
pub mod error;
pub mod message;
pub mod receiver;
pub mod router;
pub mod sender;

#[derive(Clone)]
pub struct Media {
    pub id: MediaId,
    #[allow(dead_code)]
    pub stream_id: String,
    pub peer_id: PeerId,
    #[allow(dead_code)]
    pub kind: RTPCodecType,
    pub mime_type: String,

    rtp_tx: broadcast::Sender<Arc<Packet>>,
    stopped: CancellationToken,
}

impl Media {
    pub fn new(
        peer_id: PeerId,
        id: String,
        stream_id: String,
        kind: RTPCodecType,
        mime_type: String,
        rtp_tx: broadcast::Sender<Arc<Packet>>,
        stopped: CancellationToken,
    ) -> Self {
        Self {
            id: MediaId::from(id),
            stream_id,
            peer_id,
            kind,
            mime_type,
            rtp_tx,
            stopped,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn subscribe(&self) -> Arc<dyn TrackLocal + Send + Sync> {
        let track = Arc::new(TrackLocalStaticRTP::new(
            RTCRtpCodecCapability {
                mime_type: self.mime_type.clone(),
                ..Default::default()
            },
            self.id.0.clone(),
            self.stream_id.clone(),
        ));

        let mut rtp_rx = self.rtp_tx.subscribe();
        let output_track = track.clone();
        let stopped = self.stopped.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    rtp = rtp_rx.recv() => {
                        if let Ok(rtp_packet) = rtp {
                            let _ = output_track.write_rtp(&rtp_packet).await;
                        }
                    }
                    _ = stopped.cancelled() => {
                        break;
                    }
                }
            }
        });

        track as Arc<dyn TrackLocal + Send + Sync>
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Display)]
pub struct MediaId(String);

impl From<String> for MediaId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for MediaId {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct AddMedia {
    pub media: Media,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct RemoveMedia {
    pub media_id: MediaId,
}
