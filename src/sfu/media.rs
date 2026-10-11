use crate::sfu::peer::PeerId;
use actix::Message;
use arc_swap::ArcSwap;
use derive_more::Display;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};

use crate::sfu::media::track_info::TrackInfo;
use crate::sfu::rtc::header_extension_translator::RtpHeaderExtensionTranslator;
use crate::util::id::random_id;
use rtc::rtp_transceiver::rtp_sender::{RTCRtpCodec as RTCRtpCodecCapability, RtpCodecKind};
use rtc::rtp_transceiver::{PayloadType, SSRC};
use rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication;
use rtp::packet::Packet;
use webrtc::media_stream::track_local::static_rtp::TrackLocalStaticRTP;
use webrtc::media_stream::track_local::TrackLocal;
use webrtc::media_stream::track_remote::TrackRemote;

pub mod router;
mod router_test;
pub(crate) mod track_info;

pub(crate) type RtpCodecType = RtpCodecKind;
pub(crate) type RelayRtpChannel = broadcast::Sender<Arc<Packet>>;

#[derive(Clone)]
pub(crate) struct ForwardTarget {
    peer_id: PeerId,
    track: Arc<TrackLocalStaticRTP>,
    header_extensions: RtpHeaderExtensionTranslator,
}

impl ForwardTarget {
    pub(crate) fn new(
        peer_id: PeerId,
        track: Arc<TrackLocalStaticRTP>,
        header_extensions: RtpHeaderExtensionTranslator,
    ) -> Self {
        Self {
            peer_id,
            track,
            header_extensions,
        }
    }

    fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }
}

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
    // The MID is the unique identifier of the incoming source media track.
    pub mid: String,
    #[allow(dead_code)]
    pub src_track_id: String,
    pub src_stream_id: String,
    pub header_extensions: HashMap<u8, String>,
    pub capability: RTCRtpCodecCapability,
    pub kind: RtpCodecType,
    /// The publisher SSRC is preserved on the subscriber leg for packet forwarding.
    pub ssrc: SSRC,
    source_track: Option<Arc<dyn TrackRemote>>,
    targets: Arc<ArcSwap<Vec<ForwardTarget>>>,
    pub relay_rtp_tx: RelayRtpChannel,

    // This is the muted state of a media track.
    // The state change is based on DataChannel messages. The MID is used as the identifier of
    // the media track within the messages. The flow of mute state from PeerA to PeerB is based on
    // the following sequence:
    // 1. Browser<PeerA>      -> SFU<PeerA> (Receiver): MuteMsgData { mid(receiver): String, mute: bool}
    // 2. SFU<PeerA>          -> SFU<Lobby>           : MuteMedia { peer_id: PeerId, mid(receiver): String, mute: bool}
    // 3. SFU<Lobby>          -> SFU<PeerB>           : MuteRemoteMedia { media_id: MediaId, mute: bool} // here switch to use media_id as mid
    // 4. SFU<PeerB> (Sender) -> Browser<PeerB>       : MuteMsgData { mid(sender): String, mute: bool}
    muted: bool,
    #[allow(dead_code)]
    pub purpose: MediaPurpose,
    pub info: TrackInfo,
    pub payload_type: PayloadType,
}

impl Media {
    pub fn new(
        peer_id: PeerId,
        mid: String,
        src_track_id: String,
        src_stream_id: String,
        header_extensions: HashMap<u8, String>,
        capability: RTCRtpCodecCapability,
        kind: RtpCodecType,
        ssrc: SSRC,
        source_track: Option<Arc<dyn TrackRemote>>,
        relay_rtp_tx: RelayRtpChannel,
        muted: bool,
        purpose: MediaPurpose,
        info: TrackInfo,
        payload_type: PayloadType,
    ) -> Self {
        Self {
            id: MediaId::from(peer_id.clone()),
            mid,
            src_track_id,
            src_stream_id,
            header_extensions,
            peer_id,
            kind,
            capability,
            ssrc,
            source_track,
            targets: Arc::new(ArcSwap::from_pointee(Vec::new())),
            relay_rtp_tx,
            muted,
            purpose,
            info,
            payload_type,
        }
    }

    pub(crate) fn set_mut(&mut self, muted: bool) {
        self.muted = muted;
    }

    pub(crate) fn muted(&self) -> bool {
        self.muted
    }

    pub(crate) fn add_target(&self, target: ForwardTarget) -> bool {
        let current = self.targets.load_full();
        if current
            .iter()
            .any(|existing| existing.peer_id() == target.peer_id())
        {
            return false;
        }

        let mut next = Vec::with_capacity(current.len() + 1);
        next.extend(current.iter().cloned());
        next.push(target);
        self.targets.store(Arc::new(next));
        true
    }

    pub(crate) fn remove_targets_for_peer(&self, peer_id: &PeerId) {
        let current = self.targets.load_full();
        if !current.iter().any(|target| target.peer_id() == peer_id) {
            return;
        }

        let next = current
            .iter()
            .filter(|target| target.peer_id() != peer_id)
            .cloned()
            .collect();
        self.targets.store(Arc::new(next));
    }

    pub(crate) async fn forward_rtp(&self, packet: Packet) {
        let targets = self.targets.load();
        for target in targets.iter() {
            let mut packet = packet.clone();
            target.header_extensions.translate(&mut packet);
            if let Err(err) = target.track.write_rtp(packet).await {
                log::debug!(
                    "could not forward RTP, media_id={}, peer_id={}: {err}",
                    self.id,
                    target.peer_id()
                );
            }
        }
    }

    pub(crate) async fn request_keyframe(&self) {
        if self.kind != RtpCodecType::Video {
            return;
        }

        let Some(source_track) = &self.source_track else {
            return;
        };

        if let Err(err) = source_track
            .write_rtcp(vec![Box::new(PictureLossIndication {
                sender_ssrc: 0,
                media_ssrc: self.ssrc,
            })])
            .await
        {
            log::debug!(
                "could not request keyframe, media_id={}, publisher_peer_id={}: {err}",
                self.id,
                self.peer_id
            );
        }
    }

    /// `set_remote_description` schedules local-track binding on the peer-connection driver.
    /// A bounded PLI backoff keeps a keyframe request from racing that asynchronous bind.
    pub(crate) async fn request_keyframe_after_route_activation(&self) {
        for delay in [
            Duration::ZERO,
            Duration::from_millis(200),
            Duration::from_millis(800),
        ] {
            if !delay.is_zero() {
                sleep(delay).await;
            }
            self.request_keyframe().await;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Display)]
pub struct MediaId(String);

impl MediaId {
    pub fn from(peer_id: PeerId) -> Self {
        let id_string = format!("{}-{}", random_id(5), peer_id);
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
pub(crate) struct ActivateMedia {
    pub media_id: MediaId,
    pub target: ForwardTarget,
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

#[derive(Debug, Clone, PartialEq)]
pub enum MediaPurpose {
    PARTICIPANT = 1,
    STREAM = 2,
}

impl Default for MediaPurpose {
    fn default() -> MediaPurpose {
        MediaPurpose::PARTICIPANT
    }
}

impl MediaPurpose {
    pub(crate) fn from_option_str(s: Option<&str>) -> MediaPurpose {
        match s {
            Some("1") => MediaPurpose::PARTICIPANT,
            Some("2") => MediaPurpose::STREAM,
            _ => MediaPurpose::PARTICIPANT,
        }
    }

    pub(crate) fn to_string(&self) -> String {
        let s = match self {
            MediaPurpose::PARTICIPANT => "1",
            MediaPurpose::STREAM => "2",
        };
        s.to_string()
    }
}
