use crate::sfu::peer::PeerId;
use actix::Message;
use derive_more::Display;
use enclose::enc;
use std::sync::Arc;
use tokio::sync::{broadcast, oneshot};
use tokio_util::sync::CancellationToken;

use webrtc::rtp::packet::Packet;
use webrtc::rtp_transceiver::rtp_codec::{RTCRtpCodecCapability, RTPCodecType};
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_local::TrackLocalWriter;

pub mod connector;
pub mod data_channel;
pub mod error;
pub mod message;
pub mod receiver;
pub mod router;
pub mod sender;
mod signaler;

pub(crate) type RtpSenderChannel = broadcast::Sender<Arc<Packet>>;

#[derive(Clone)]
pub struct Media {
    pub id: MediaId,
    #[allow(dead_code)]
    pub stream_id: String,
    pub peer_id: PeerId,
    pub capability: RTCRtpCodecCapability,
    #[allow(dead_code)]
    pub kind: RTPCodecType,
    rtp_tx: RtpSenderChannel,
    stopped: CancellationToken,
}

impl Media {
    pub fn new(
        peer_id: PeerId,
        id: String,
        stream_id: String,
        capability: RTCRtpCodecCapability,
        kind: RTPCodecType,
        rtp_tx: broadcast::Sender<Arc<Packet>>,
        stopped: CancellationToken,
    ) -> Self {
        Self {
            id: MediaId::from(id),
            stream_id,
            peer_id,
            kind,
            capability,
            rtp_tx,
            stopped,
        }
    }

    pub(crate) async fn subscribe(&self, local_track: Arc<TrackLocalStaticRTP>) {
        let mut rtp_rx = self.rtp_tx.subscribe();
        let publisher_stopped = self.stopped.clone();

        let (started_tx, started_rx) = oneshot::channel();
        tokio::spawn(enc!( (local_track )  async move {
            started_tx.send(()).unwrap();
            loop {
                tokio::select! {
                    rtp = rtp_rx.recv() => {
                        match rtp {
                            Ok(rtp_packet) => {
                                let _ = local_track.write_rtp(&rtp_packet).await;
                            },
                            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                                log::warn!("rtp_tx.recv() skipped {} packets", skipped);
                            }
                            Err(_) => break,
                        }
                    }
                    _ = publisher_stopped.cancelled() => {
                        break;
                    }
                }
            }
        }));
        let _ = started_rx.await;
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
