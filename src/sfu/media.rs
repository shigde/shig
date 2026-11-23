use crate::sfu::peer::PeerId;
use actix::Message;
use arc_swap::ArcSwap;
use derive_more::Display;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;

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

pub mod config;
pub mod media_service;
pub mod recording;
mod hls_writer;
pub mod router;
pub(crate) mod track_info;
mod pli_gate;
mod forward_queue;

pub use media_service::MediaService;
pub use recording::Recording;

#[cfg(test)]
mod router_test;

pub(crate) type RtpCodecType = RtpCodecKind;
pub(crate) type RelayRtpChannel = broadcast::Sender<Arc<Packet>>;

#[derive(Clone)]
pub(crate) struct ForwardTarget {
    peer_id: PeerId,
    track: Arc<TrackLocalStaticRTP>,
    payload_type: PayloadType,
    header_extensions: RtpHeaderExtensionTranslator,
    rtp_binding_confirmed: Arc<AtomicBool>,
    feedback_cancel: tokio_util::sync::CancellationToken,
    queue: Arc<forward_queue::ForwardQueue>,
}

impl ForwardTarget {
    pub(crate) fn new(
        peer_id: PeerId,
        track: Arc<TrackLocalStaticRTP>,
        payload_type: PayloadType,
        header_extensions: RtpHeaderExtensionTranslator,
    ) -> Self {
        let queue = Arc::new(forward_queue::ForwardQueue::new());
        Self {
            peer_id,
            track,
            payload_type,
            header_extensions,
            rtp_binding_confirmed: Arc::new(AtomicBool::new(false)),
            feedback_cancel: queue.cancel.clone(),
            queue,
        }
    }

    fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }

    pub(crate) fn feedback_cancel(&self) -> tokio_util::sync::CancellationToken {
        self.feedback_cancel.clone()
    }

    /// Only the first successful write starts feedback and requests a keyframe.
    fn confirm_rtp_binding(binding: &AtomicBool) -> bool {
        binding.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_ok()
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
    subscriber_pli_gate: Arc<pli_gate::PliGate>,
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
            subscriber_pli_gate: Arc::new(pli_gate::PliGate::default()),
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

    pub(crate) fn forward_rtp(&self, packet: Packet) {
        if packet.header.payload_type != self.payload_type {
            return;
        }
        let targets = self.targets.load();
        if targets.is_empty() { return; }
        let packet = Arc::new(packet);
        for target in targets.iter() {
            self.start_forward_sender(target);
            target.queue.enqueue(packet.clone());
        }
    }

    fn keyframe_source(&self) -> KeyframeSource {
        KeyframeSource {
            id: self.id.clone(), peer_id: self.peer_id.clone(), kind: self.kind,
            ssrc: self.ssrc, source_track: self.source_track.clone(),
            subscriber_pli_gate: self.subscriber_pli_gate.clone(),
        }
    }

    fn start_forward_sender(&self, target: &ForwardTarget) {
        let Some(mut receiver) = target.queue.take_receiver() else { return; };
        let track = target.track.clone();
        let payload_type = target.payload_type;
        let header_extensions = target.header_extensions.clone();
        let bound = target.rtp_binding_confirmed.clone();
        let cancel = target.feedback_cancel.clone();
        let subscriber = target.peer_id.clone();
        let source = self.keyframe_source();
        tokio::spawn(async move {
          let mut diagnostics = ForwardDiagnostics::default();
          let mut report = tokio::time::interval_at(
              tokio::time::Instant::now() + std::time::Duration::from_secs(1),
              std::time::Duration::from_secs(1),
          );
          report.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
          loop {
            let packet = tokio::select! {
                biased;
                _ = cancel.cancelled() => break,
                _ = report.tick() => {
                    diagnostics.report(&source.id, &subscriber, receiver.len(), None);
                    continue;
                },
                packet = receiver.recv() => match packet {
                    Ok(packet) => packet,
                    Err(broadcast::error::RecvError::Lagged(count)) => {
                        diagnostics.dropped += count;
                        continue;
                    },
                    Err(broadcast::error::RecvError::Closed) => break,
                },
            };
            diagnostics.observe_depth(receiver.len());
            let mut packet = packet.as_ref().clone();
            packet.header.payload_type = payload_type;
            header_extensions.translate(&mut packet);
            let started = std::time::Instant::now();
            let write = track.write_rtp(packet);
            tokio::pin!(write);
            let result = loop {
                tokio::select! {
                    biased;
                    _ = cancel.cancelled() => return,
                    _ = report.tick() => {
                        diagnostics.report(&source.id, &subscriber, receiver.len(), Some(started.elapsed()));
                    },
                    result = &mut write => break result,
                }
            };
            diagnostics.observe_write(started.elapsed(), result.is_err());
            match result {
                Ok(()) => {
                    if source.kind == RtpCodecType::Video
                        && ForwardTarget::confirm_rtp_binding(&bound) {
                        source.start_subscriber_feedback(track.clone(), subscriber.clone(), cancel.clone());
                        log::info!(
                            "RTP target bound, requesting keyframe, media_id={}, subscriber_peer_id={}",
                            source.id,
                            subscriber
                        );
                        tokio::select! {
                            biased;
                            _ = cancel.cancelled() => break,
                            _ = source.request_keyframe(&subscriber) => {},
                        }
                    }
                }
                Err(err) => {
                    log::debug!(
                        "could not forward RTP, media_id={}, peer_id={}: {err}",
                        source.id,
                        subscriber
                    );
                }
            }
          }
        });
    }

    pub(crate) async fn request_keyframe(&self, subscriber_peer_id: &PeerId) {
        self.keyframe_source().request_keyframe(subscriber_peer_id).await;
    }
}

/// One-second diagnostic windows, local to a subscriber sender task.
#[derive(Default)]
struct ForwardDiagnostics {
    writes: u64,
    errors: u64,
    dropped: u64,
    max_depth: usize,
    total_write: std::time::Duration,
    max_write: std::time::Duration,
}

impl ForwardDiagnostics {
    fn observe_depth(&mut self, pending: usize) {
        self.max_depth = self.max_depth.max(pending.min(forward_queue::CAPACITY));
    }

    fn observe_write(&mut self, elapsed: std::time::Duration, failed: bool) {
        self.writes += 1;
        self.errors += u64::from(failed);
        self.total_write += elapsed;
        self.max_write = self.max_write.max(elapsed);
    }

    fn report(&mut self, media: &MediaId, subscriber: &PeerId, pending: usize,
        in_flight: Option<std::time::Duration>) {
        self.observe_depth(pending);
        log::debug!(
            "RTP forward queue: media_id={}, subscriber_peer_id={}, queue_pending={}, queue_capacity={}, queue_max_observed={}, queue_dropped={}, writes_completed={}, write_errors={}, write_avg_ms={:.3}, write_max_ms={:.3}, write_pending_ms={:.3}",
            media, subscriber, pending.min(forward_queue::CAPACITY), forward_queue::CAPACITY,
            self.max_depth, self.dropped, self.writes, self.errors,
            if self.writes == 0 { 0.0 } else { self.total_write.as_secs_f64() * 1000.0 / self.writes as f64 },
            self.max_write.as_secs_f64() * 1000.0,
            in_flight.unwrap_or_default().as_secs_f64() * 1000.0,
        );
        *self = Self::default();
    }
}

#[cfg(test)]
mod forward_diagnostics_tests {
    use super::*;

    #[test]
    fn diagnostics_bound_depth_and_include_failed_write_time() {
        let mut stats = ForwardDiagnostics::default();
        stats.observe_depth(600);
        stats.observe_depth(10);
        stats.observe_write(std::time::Duration::from_millis(5), false);
        stats.observe_write(std::time::Duration::from_millis(25), true);
        assert_eq!(stats.max_depth, 512);
        assert_eq!(stats.writes, 2);
        assert_eq!(stats.errors, 1);
        assert_eq!(stats.total_write, std::time::Duration::from_millis(30));
        assert_eq!(stats.max_write, std::time::Duration::from_millis(25));
    }
}

/// Feedback state deliberately excludes routes and their queues to avoid task cycles.
#[derive(Clone)]
struct KeyframeSource {
    id: MediaId,
    peer_id: PeerId,
    kind: RtpCodecType,
    ssrc: SSRC,
    source_track: Option<Arc<dyn TrackRemote>>,
    subscriber_pli_gate: Arc<pli_gate::PliGate>,
}

impl KeyframeSource {
    fn start_subscriber_feedback(&self, track: Arc<TrackLocalStaticRTP>, subscriber: PeerId,
        cancel: tokio_util::sync::CancellationToken) {
        use webrtc::media_stream::track_local::TrackLocalEvent;
        let source = self.clone();
        tokio::spawn(async move {
            loop {
                let event = tokio::select! {
                    biased;
                    _ = cancel.cancelled() => break,
                    event = track.poll() => event,
                };
                let Some(TrackLocalEvent::OnRtcpPacket(packets)) = event else { break; };
                let requests_source = crate::sfu::rtc::subscriber_pli::requests_ssrc(&packets, source.ssrc);
                if requests_source {
                    tokio::select! {
                        biased;
                        _ = cancel.cancelled() => break,
                        _ = source.request_keyframe(&subscriber) => {},
                    }
                }
            }
        });
    }

    pub(crate) async fn request_keyframe(&self, subscriber_peer_id: &PeerId) {
        if self.kind != RtpCodecType::Video {
            return;
        }

        let Some(source_track) = &self.source_track else {
            log::info!(
                "cannot request keyframe without source track, media_id={}, subscriber_peer_id={}",
                self.id,
                subscriber_peer_id
            );
            return;
        };

        // All request sources share this gate: client feedback, live rhythm,
        // subscription startup and relay recovery/cache protection.
        if !self.subscriber_pli_gate.admit(std::time::Instant::now()) {
            return;
        }

        match source_track
            .write_rtcp(vec![Box::new(PictureLossIndication {
                sender_ssrc: 0,
                media_ssrc: self.ssrc,
            })])
            .await
        {
            Ok(()) => log::info!(
                "keyframe request queued, media_id={}, publisher_peer_id={}, subscriber_peer_id={}, media_ssrc={}",
                self.id,
                self.peer_id,
                subscriber_peer_id,
                self.ssrc
            ),
            Err(err) => log::info!(
                "could not request keyframe, media_id={}, publisher_peer_id={}: {err}",
                self.id,
                self.peer_id
            ),
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
