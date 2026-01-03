use crate::sfu::peer::PeerId;
use actix::Message;
use derive_more::Display;
use enclose::enc;
use std::borrow::Borrow;
use std::sync::Arc;
use tokio::sync::{broadcast, oneshot};
use tokio_util::sync::CancellationToken;

use crate::util::id::random_id;
use webrtc::rtp::packet::Packet;
use webrtc::rtp_transceiver::rtp_codec::{RTCRtpCodecCapability, RTPCodecType};
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_local::TrackLocalWriter;

pub mod connector;
pub mod data_channel;
mod data_channel_test;
pub mod error;
pub mod message;
pub mod receiver;
pub mod router;
mod router_test;
mod sdp;
pub mod sender;
mod signaler;

pub(crate) type RtpSenderChannel = broadcast::Sender<Arc<Packet>>;

/**
 * The Media object is centrally managed in the lobby.
 * Media represents a single media track (audio or video) that is being sent from a browser ore
 * other client to the SFU.
 *
 * Media is created by a receiver from an incoming track of a peer.
 * Via the lobby, it is passed on to a sender of another peer.
 */
#[derive(Clone)]
pub struct Media {
    pub id: MediaId,
    pub peer_id: PeerId,
    pub mid: String,
    #[allow(dead_code)]
    pub src_track_id: String,
    pub src_stream_id: String,
    pub capability: RTCRtpCodecCapability,
    #[allow(dead_code)]
    pub kind: RTPCodecType,
    rtp_tx: RtpSenderChannel,
    stopped: CancellationToken,

    // This is the muted state of a media track.
    // The state change is based on DataChannel messages. The MID is used as the identifier of
    // the media track within the messages. The flow of mute state from PeerA to PeerB is based on
    // the following sequence:
    // 1. Browser<PeerA>      -> SFU<PeerA> (Receiver): MuteMsgData { mid(resv): String, mute: bool}
    // 2. SFU<PeerA>          -> SFU<Lobby>           : MuteMedia { peer_id: PeerId, mid(resv): String, mute: bool}
    // 3. SFU<Lobby>          -> SFU<PeerB>           : MuteRemoteMedia { media_id: MediaId, mute: bool}
    // 4. SFU<PeerB> (Sender) -> Browser<PeerB>       : MuteMsgData { mid(send): String, mute: bool}
    muted: bool,
}

impl Media {
    pub fn new(
        peer_id: PeerId,
        mid: String,
        src_track_id: String,
        src_stream_id: String,
        capability: RTCRtpCodecCapability,
        kind: RTPCodecType,
        rtp_tx: broadcast::Sender<Arc<Packet>>,
        stopped: CancellationToken,
    ) -> Self {
        Self {
            id: MediaId::from(peer_id.clone()),
            mid,
            src_track_id,
            src_stream_id,
            peer_id,
            kind,
            capability,
            rtp_tx,
            stopped,
            muted: false,
        }
    }

    pub(crate) fn set_mut(&mut self, muted: bool) {
        self.muted = muted;
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

impl MediaId {
    pub fn from(peer_id: PeerId) -> Self {
        let id_string = format!("{}-{}", random_id(6), peer_id);
        log::trace!("MediaId: ({}", id_string);
        Self(id_string)
    }
}

#[allow(dead_code)]
impl From<String> for MediaId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

#[allow(dead_code)]
impl From<&str> for MediaId {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl Borrow<str> for MediaId {
    fn borrow(&self) -> &str {
        &self.0
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

#[derive(Message)]
#[rtype(result = "()")]
pub struct MuteMedia {
    pub peer_id: PeerId,
    pub mid: String,
    pub mute: bool,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct MuteRemoteMedia {
    pub media_id: MediaId,
    pub mute: bool,
}
