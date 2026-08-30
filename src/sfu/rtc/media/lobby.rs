use super::control::{
    parse_control_message, ControlMessage, ControlMetadata, ControlOutMessage, ControlRouter,
    ControlSend,
};
use super::demuxer::Demuxer;
use super::endpoint::{
    Mid, PublishedTrack, PublishedTrackInfo, RtcEndpoint, RtcEndpointBuilder, RtcEndpointEvent,
};
use super::event::SFUEvent;
use super::forward::{ForwardKey, ForwardTable, ForwardTarget, HeaderExtensionRewrite};
use super::rtcp_forwarder::RtcpForwarderBuilder;
use super::RtcDiagnostics;
use crate::metrics;
use crate::metrics::PeerConnectionStats;
use crate::sfu::endpoint::{EndpointId, EndpointKind, RtcEndpointId};
use base64::engine::general_purpose::STANDARD_NO_PAD;
use base64::Engine;
use log::{debug, trace, warn};
use rtc::ice::rand::{generate_pwd, generate_ufrag};
use rtc::interceptor::Registry;
use rtc::peer_connection::configuration::interceptor_registry::register_default_interceptors;
use rtc::peer_connection::configuration::media_engine::MediaEngine;
use rtc::peer_connection::configuration::setting_engine::SettingEngine;
use rtc::peer_connection::event::{RTCDataChannelEvent, RTCPeerConnectionEvent, RTCTrackEvent};
use rtc::peer_connection::message::RTCMessage;
use rtc::peer_connection::sdp::{RTCSdpType, RTCSessionDescription};
use rtc::peer_connection::transport::RTCDtlsRole;
use rtc::rtcp::header::{PacketType, FORMAT_CCFB};
use rtc::rtcp::payload_feedbacks::full_intra_request::FullIntraRequest;
use rtc::rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication;
use rtc::rtcp::receiver_report::ReceiverReport;
use rtc::rtcp::sender_report::SenderReport;
use rtc::rtcp::transport_feedbacks::transport_layer_nack::TransportLayerNack;
use rtc::rtcp::Packet;
use rtc::rtp_transceiver::rtp_sender::{
    RTCRtpCodec, RTCRtpCodecParameters, RTCRtpHeaderExtensionParameters, RTCRtpSendParameters,
    RtpCodecKind,
};
use rtc::sdp::extmap::SDES_MID_URI;
use rtc::shared::error::{flatten_errs, Error};
use rtc::shared::TaggedBytesMut;
use sansio::Protocol;
use std::collections::{HashMap, HashSet, VecDeque};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::time::Instant;
use uuid::Uuid;

/// Lobbies are identified by a UUID minted by the signaling authority. The SFU treats it as
/// an opaque, `Copy` identifier: it never interprets the layout, and only requires a
/// rendering that is legal inside an ICE ufrag (see [`RtcLobby::build_endpoint`]).
pub type RtcLobbyId = Uuid;

/// Render a lobby id as the string that appears as base64, unpadded.
fn encode_rtc_lobby_id(rtc_lobby_id: &RtcLobbyId) -> String {
    STANDARD_NO_PAD.encode(rtc_lobby_id.as_bytes())
}

/// Parse a lobby id from base64, unpadded.
fn decode_rtc_lobby_id(token: &str) -> Option<RtcLobbyId> {
    let bytes = STANDARD_NO_PAD.decode(token).ok()?;
    let bytes: [u8; 16] = bytes.try_into().ok()?;
    let rtc_lobby_id = Uuid::from_bytes(bytes);

    Some(rtc_lobby_id)
}

// The SFU has one UDP socket per media shard, so an arriving packet has to say which lobby
// and peer it belongs to. ICE gives us exactly one field to carry that: the ufrag the
// browser echoes in every STUN binding request. These two functions are the only places
// that know its layout, and they must stay inverses of each other.
//
//   USERNAME        = local_ufrag ":" remote_ufrag
//   ufrag           = 4*256ice-char                  // length range [4, 256]
//   ice-char        = ALPHA / DIGIT / "+" / "/"
//   local_ufrag     = base64_lobby_id "/" digit_peer_id "+" alpha_ufrag
//   base64_lobby_id  = ALPHA / DIGIT / "+" / "/"
//   digit_peer_id = DIGIT
//   alpha_ufrag     = ALPHA

/// Build the local ufrag that names `rtc_lobby_id`/`endpoint_id`, with a random suffix so two
/// endpoints of one lobby never share credentials.
pub(crate) fn encode_local_ufrag(rtc_lobby_id: &RtcLobbyId, endpoint_id: RtcEndpointId) -> String {
    format!(
        "{}/{}+{}",
        encode_rtc_lobby_id(rtc_lobby_id), // ALPHA / DIGIT / "+" / "/"
        endpoint_id,
        generate_ufrag()
    )
}

/// Recover the lobby and peer a local ufrag names, or `None` if it was not built by
/// [`encode_local_ufrag`].
///
/// Both separators can also occur *inside* the base64 lobby id, so the split works from
/// the right: the last `+` is the one before the alphabetic suffix, and the last `/` the
/// one before the decimal peer id. Splitting from the left would truncate the lobby id.
pub(crate) fn decode_local_ufrag(local_ufrag: &str) -> Option<(RtcLobbyId, RtcEndpointId)> {
    let (lobby_str, peer_str) = local_ufrag.rsplit_once('+')?.0.rsplit_once('/')?;
    Some((decode_rtc_lobby_id(lobby_str)?, peer_str.parse().ok()?))
}

pub(crate) struct RtcLobby {
    id: RtcLobbyId,
    local_addr: SocketAddr,
    demuxer: Demuxer,
    diagnostics: RtcDiagnostics,
    endpoints: HashMap<RtcEndpointId, RtcEndpoint>,
    publishers: HashSet<RtcEndpointId>,
    subscribers: HashSet<RtcEndpointId>,
    published_tracks: HashMap<ForwardKey, PublishedTrackInfo>,
    forward: ForwardTable,
    control: ControlRouter,

    writes: VecDeque<TaggedBytesMut>,
    events: VecDeque<SFUEvent>,
}

impl RtcLobby {
    pub(crate) fn new(
        id: RtcLobbyId,
        local_addr: SocketAddr,
        diagnostics: RtcDiagnostics,
    ) -> Self {
        Self {
            id,
            local_addr,

            demuxer: Default::default(),
            diagnostics,
            endpoints: Default::default(),
            publishers: Default::default(),
            subscribers: Default::default(),
            published_tracks: Default::default(),
            forward: Default::default(),
            control: Default::default(),
            writes: Default::default(),
            events: Default::default(),
        }
    }

    pub(crate) fn id(&self) -> RtcLobbyId {
        self.id
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.endpoints.is_empty()
    }

    pub(crate) fn endpoint_count(&self) -> usize {
        self.endpoints.len()
    }

    pub(crate) fn publisher_count(&self) -> usize {
        self.publishers.len()
    }

    pub(crate) fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }

    pub(crate) fn route_count(&self) -> usize {
        self.forward.route_count()
    }

    pub(crate) fn collect_peer_connection_stats(
        &mut self,
        now: Instant,
        stats: &mut PeerConnectionStats,
    ) {
        for endpoint in self.endpoints.values_mut() {
            endpoint.collect_peer_connection_stats(now, stats);
        }
    }

    /// Build a peer with the default media engine (default codecs), the default
    /// interceptor chain, and default setting engine.
    fn build_endpoint(
        &self,
        endpoint_id: EndpointId,
        rtc_lobby_id: RtcLobbyId,
    ) -> Result<RtcEndpoint, Error> {
        let mut setting_engine = SettingEngine::default();
        setting_engine.set_ice_credentials(
            encode_local_ufrag(&rtc_lobby_id, endpoint_id.rtc_id()),
            generate_pwd(),
        );
        setting_engine.set_lite(true);
        // The SFU is ICE-lite (controlled) and DTLS-passive: it answers `a=setup:passive`
        // so the browser is the DTLS peer and initiates the handshake (sends the
        // ClientHello) once ICE connects. Without this, the answer defaults to
        // `a=setup:active` (DTLS endpoint) — a mismatch that deadlocks the handshake.
        setting_engine.set_answering_dtls_role(RTCDtlsRole::Server)?;
        let mut media_engine = MediaEngine::default();
        media_engine.register_default_codecs()?;
        let registry = register_default_interceptors(Registry::new(), &mut media_engine)?;
        // Outermost layer: surface inbound RTCP (a subscriber's PLI/FIR keyframe requests)
        // to poll_read so the SFU can relay them upstream to the publisher; the default
        // chain would otherwise consume RTCP before the application sees it.
        let role = match endpoint_id.kind() {
            EndpointKind::Publish => "publish",
            EndpointKind::Subscribe => "subscribe",
        };
        let registry = registry.with(
            RtcpForwarderBuilder::new(role, self.diagnostics.nack_cache).build(),
        );
        RtcEndpointBuilder::new(
            endpoint_id,
            rtc_lobby_id,
            self.local_addr,
            self.diagnostics.packet_io,
        )
            .with_setting_engine(setting_engine)
            .with_media_engine(media_engine)
            .with_interceptor_registry(registry)
            .build()
    }

    /// Reconcile the forwarding graph with the lobby's current publish state.
    ///
    /// For every publisher's live publish track, each *other* endpoint should have exactly
    /// one forwarding sender. This diffs that desired matrix against the `ForwardTable`
    /// (keyed by `{publisher, mid}`) and only applies the delta:
    ///   - subscribers that left, or tracks no longer published → `remove_forwarding_track`,
    ///   - `(publisher, mid, subscriber)` cells not yet present → `add_forwarding_track`,
    ///   - everything already wired → left untouched.
    ///
    /// It is therefore idempotent: calling it after a publisher re-offers the same tracks
    /// adds nothing. Run it whenever publish state may have changed (join / leave / an
    /// applied session description).
    fn reconcile(&mut self) {
        let started_at = Instant::now();
        // Snapshot publish state before mutating any endpoint. `get_forward_tracks` reads
        // the negotiated receivers (needs `&mut`), so this collects owned tracks first
        // and releases the borrow before the add/remove passes below.
        let live_publishers: HashSet<RtcEndpointId> = self
            .publishers
            .iter()
            .copied()
            .filter(|id| self.endpoints.contains_key(id))
            .collect();
        let live_subscribers: HashSet<RtcEndpointId> = self
            .subscribers
            .iter()
            .copied()
            .filter(|id| self.endpoints.contains_key(id))
            .collect();
        let publishers: Vec<(RtcEndpointId, HashMap<Mid, PublishedTrack>)> = live_publishers
            .iter()
            .filter_map(|id| {
                self.endpoints
                    .get_mut(id)
                    .map(|endpoint| (*id, endpoint.get_forward_tracks()))
            })
            .filter(|(_, tracks)| !tracks.is_empty())
            .collect();
        let mut published_tracks = HashMap::new();

        let mut desired = HashSet::new();
        for (publisher, tracks) in &publishers {
            let Some(publisher_peer) = self
                .endpoints
                .get(publisher)
                .map(|endpoint| endpoint.id().peer_id().clone())
            else {
                continue;
            };
            for mid in tracks.keys() {
                desired.insert(ForwardKey {
                    publisher_peer: publisher_peer.clone(),
                    publish_endpoint: *publisher,
                    mid: mid.clone(),
                });
            }
        }

        // 1. Tear down forwardings that are no longer wanted.
        let mut removed = Vec::new();
        self.forward.retain(
            &desired,
            &live_publishers,
            &live_subscribers,
            &mut removed,
        );
        let removed_routes = removed.len() as u64;
        for (subscriber, sender) in removed {
            if let Some(endpoint) = self.endpoints.get_mut(&subscriber) {
                if let Err(err) = endpoint.remove_forward_track(sender) {
                    warn!("{}: failed to remove forwarding sender: {}", self.id, err);
                }
            }
        }
        let mut added_routes = 0_u64;

        // 2. Add the forwardings that are missing. The publisher's track
        //    is forwarded verbatim onto a sendonly transceiver per subscriber.
        for (publisher, tracks) in &publishers {
            let Some(publisher_peer) = self
                .endpoints
                .get(publisher)
                .map(|endpoint| endpoint.id().peer_id().clone())
            else {
                continue;
            };

            for (mid, track) in tracks {
                let key = ForwardKey {
                    publisher_peer: publisher_peer.clone(),
                    publish_endpoint: *publisher,
                    mid: mid.clone(),
                };
                published_tracks.insert(key.clone(), track.info.clone());

                // Bind the publisher's wire SSRC(s) for packet routing (idempotent).
                // Tracks whose SSRC the SDP couldn't name (bare m-line, RID simulcast)
                // are bound later from the publisher's OnTrack(OnOpen) in poll_event.
                for ssrc in track.track.ssrcs() {
                    self.forward.bind_ssrc(ssrc, key.clone());
                }

                for &subscriber in &live_subscribers {
                    let Some(subscriber_peer) = self
                        .endpoints
                        .get(&subscriber)
                        .map(|endpoint| endpoint.id().peer_id().clone())
                    else {
                        continue;
                    };
                    if subscriber_peer == publisher_peer {
                        continue;
                    }
                    if let Some(sender) = self.forward.subscriber_sender(&key, &subscriber_peer) {
                        if let Some(endpoint) = self.endpoints.get_mut(&subscriber) {
                            if let Some(target) = RtcLobby::build_forward_target(
                                endpoint,
                                subscriber_peer,
                                subscriber,
                                sender,
                                track,
                            ) {
                                self.forward.update_subscriber_target(&key, target);
                            }
                        }
                        continue;
                    }

                    if let Some(endpoint) = self.endpoints.get_mut(&subscriber) {
                        match endpoint.add_forward_track(track.track.clone(), track.info.clone()) {
                            Ok(sender) => {
                                if let Some(target) = RtcLobby::build_forward_target(
                                    endpoint,
                                    subscriber_peer,
                                    subscriber,
                                    sender,
                                    track,
                                ) {
                                    self.forward.insert(key.clone(), target);
                                    added_routes += 1;
                                }
                            }
                            Err(err) => warn!(
                                "{}: failed to add forwarding {}->{} for mid {}: {}",
                                self.id, publisher, subscriber, mid, err
                            ),
                        }
                    }
                }
            }
        }
        self.published_tracks = published_tracks;
        metrics::observe_reconcile(
            started_at.elapsed().as_secs_f64(),
            added_routes,
            removed_routes,
        );
    }

    fn build_forward_target(
        endpoint: &mut RtcEndpoint,
        subscriber_peer: crate::sfu::peer::PeerId,
        subscribe_endpoint: RtcEndpointId,
        sender_id: rtc::rtp_transceiver::RTCRtpSenderId,
        track: &PublishedTrack,
    ) -> Option<ForwardTarget> {
        let sender_parameters = endpoint.sender_parameters(sender_id)?;
        let payload_types = RtcLobby::payload_type_map(&track.codecs, &sender_parameters);
        debug!(
            "forward target codecs subscriber={} sender={:?} publisher={:?} subscriber={:?} payload_map={:?}",
            subscribe_endpoint,
            sender_id,
            RtcLobby::codec_summary(&track.codecs),
            RtcLobby::codec_summary(&sender_parameters.rtp_parameters.codecs),
            payload_types
        );
        Some(ForwardTarget {
            subscriber_peer,
            subscribe_endpoint,
            sender_id,
            payload_types,
            extension_rewrites: RtcLobby::header_extension_rewrites(
                &track.header_extensions,
                &sender_parameters.rtp_parameters.header_extensions,
            ),
            subscriber_mid: endpoint.transceiver_mid(sender_id),
        })
    }

    fn payload_type_map(
        publisher_codecs: &[RTCRtpCodecParameters],
        sender_parameters: &RTCRtpSendParameters,
    ) -> HashMap<u8, u8> {
        publisher_codecs
            .iter()
            .filter_map(|publisher_codec| {
                RtcLobby::payload_type_for_codec(
                    &sender_parameters.rtp_parameters.codecs,
                    &publisher_codec.rtp_codec,
                )
                .map(|subscriber_payload_type| {
                    (publisher_codec.payload_type, subscriber_payload_type)
                })
            })
            .collect()
    }

    fn payload_type_for_codec(
        subscriber_codecs: &[RTCRtpCodecParameters],
        publisher_codec: &RTCRtpCodec,
    ) -> Option<u8> {
        subscriber_codecs
            .iter()
            .find(|candidate| {
                candidate
                    .rtp_codec
                    .mime_type
                    .eq_ignore_ascii_case(&publisher_codec.mime_type)
                    && candidate.rtp_codec.sdp_fmtp_line == publisher_codec.sdp_fmtp_line
            })
            .or_else(|| {
                subscriber_codecs.iter().find(|candidate| {
                    candidate
                        .rtp_codec
                        .mime_type
                        .eq_ignore_ascii_case(&publisher_codec.mime_type)
                })
            })
            .map(|matched| matched.payload_type)
    }

    fn codec_summary(codecs: &[RTCRtpCodecParameters]) -> Vec<(u8, String, String)> {
        codecs
            .iter()
            .map(|codec| {
                (
                    codec.payload_type,
                    codec.rtp_codec.mime_type.clone(),
                    codec.rtp_codec.sdp_fmtp_line.clone(),
                )
            })
            .collect()
    }

    fn header_extension_rewrites(
        publisher_extensions: &[RTCRtpHeaderExtensionParameters],
        subscriber_extensions: &[RTCRtpHeaderExtensionParameters],
    ) -> Vec<HeaderExtensionRewrite> {
        publisher_extensions
            .iter()
            .filter_map(|publisher_extension| {
                subscriber_extensions
                    .iter()
                    .find(|subscriber_extension| subscriber_extension.uri == publisher_extension.uri)
                    .map(|subscriber_extension| HeaderExtensionRewrite {
                        publisher_id: publisher_extension.id as u8,
                        subscriber_id: subscriber_extension.id as u8,
                        rewrite_mid: subscriber_extension.uri == SDES_MID_URI,
                    })
            })
            .collect()
    }

    /// Build the RTP packet to forward to one subscriber.
    ///
    /// Clones `rtp_packet`, rewrites its payload type to the subscriber's negotiated
    /// `outbound_payload_type`, and translates each RTP header extension id from the publisher's
    /// negotiated id to the subscriber's — matching extensions by uri, dropping any the subscriber
    /// did not negotiate, and stamping the subscriber's own m-line `mid` as the payload of the
    /// `sdes:mid` extension. The header-extension translation only runs when *both* the publisher and
    /// the subscriber negotiated header extensions for this stream; otherwise the cloned packet's
    /// extensions are forwarded untouched.
    fn translate_rtp_for_subscriber(
        rtp_packet: &rtc::rtp::Packet,
        outbound_payload_type: u8,
        extension_rewrites: &[HeaderExtensionRewrite],
        subscriber_mid: Option<&str>,
    ) -> rtc::rtp::Packet {
        let mut forwarded_rtp = rtp_packet.clone();
        forwarded_rtp.header.payload_type = outbound_payload_type;

        if !extension_rewrites.is_empty() {
            // Map each extension the packet carries to the subscriber's negotiated id, matching by
            // uri: publisher id -> uri (pub_exts) -> subscriber id (sub_exts). Extensions the
            // subscriber didn't negotiate are dropped; the sdes:mid payload is replaced with the
            // subscriber's own mid.
            let mut translated: Vec<(u8, ::bytes::Bytes)> = Vec::new();
            let ids = forwarded_rtp.header.get_extension_ids();
            for id in &ids {
                let Some(payload) = forwarded_rtp.header.get_extension(*id) else {
                    continue;
                };
                let Some(rewrite) = extension_rewrites
                    .iter()
                    .find(|rewrite| rewrite.publisher_id == *id)
                else {
                    continue;
                };
                let payload = if rewrite.rewrite_mid {
                    if let Some(mid) = subscriber_mid {
                        ::bytes::Bytes::copy_from_slice(mid.as_bytes())
                    } else {
                        payload
                    }
                } else {
                    payload
                };
                // TODO: If we ever support forwarding multiple simulcast layers to a subscriber
                //  that negotiated RID/RRID, we would need to map the publisher's RID/RRID payload
                //  values to the subscriber's corresponding layer IDs here.
                //  "urn:ietf:params:rtp-hdrext:sdes:rtp-stream-id",
                //  "urn:ietf:params:rtp-hdrext:sdes:repaired-rtp-stream-id",
                translated.push((rewrite.subscriber_id, payload));
            }

            // Rebuild the extensions through the public API so `extensions_padding` and the
            // extension flag stay consistent — a direct `header.extensions = …` assignment leaves
            // them stale and breaks marshalling. Clear all first to avoid id collisions during the
            // remap (an old id may be another extension's new id).
            for id in ids {
                let _ = forwarded_rtp.header.del_extension(id);
            }
            for (id, payload) in translated {
                let _ = forwarded_rtp.header.set_extension(id, payload);
            }
        }

        forwarded_rtp
    }

    /// Forward one publisher RTP packet to every subscriber bound to its SSRC, translating the
    /// payload type and header extension ids for each subscriber leg (see
    /// [`translate_rtp_for_subscriber`]). Drops the packet if its SSRC is not (yet) bound, or if
    /// it arrives from an endpoint other than the SSRC's bound publisher.
    fn forward_rtp(&mut self, endpoint_id: RtcEndpointId, rtp_packet: &rtc::rtp::Packet) {
        metrics::inc_rtp_in(rtp_packet.payload.len());
        if !self.publishers.contains(&endpoint_id) {
            metrics::inc_rtp_dropped("non_publisher");
            trace!(
                "{}: dropping rtp from non-publish endpoint {}",
                self.id,
                endpoint_id
            );
            return;
        }

        let ssrc = rtp_packet.header.ssrc;
        let Some((key, subscribers)) = self.forward.route_by_ssrc(ssrc) else {
            metrics::inc_rtp_dropped("no_route");
            trace!(
                "{}: no forward binding for rtp ssrc {} from {}",
                self.id,
                ssrc,
                endpoint_id
            );
            return;
        };
        if key.publish_endpoint != endpoint_id {
            metrics::inc_rtp_dropped("wrong_publisher");
            warn!(
                "{}: rtp ssrc {} from {} is bound to publisher {} — dropping",
                self.id, ssrc, endpoint_id, key.publish_endpoint
            );
            return;
        }

        let forward_started_at = self.diagnostics.forward_timing.then(Instant::now);
        let subscriber_count = subscribers.len();
        let media = self
            .published_tracks
            .get(key)
            .map(|track| media_kind_label(track.media_kind))
            .unwrap_or("unknown");
        let inbound_payload_type = rtp_packet.header.payload_type;

        for target in subscribers.values() {
            let subscriber = target.subscribe_endpoint;
            let sender_id = target.sender_id;
            let Some(endpoint) = self.endpoints.get_mut(&subscriber) else {
                continue;
            };
            // Skip subscribers whose transport isn't up yet: the SRTP context isn't set until
            // DTLS completes, so forwarding now would just be dropped ("local_srtp_context is not
            // set yet"). Once connected, the subscriber requests a keyframe and media flows.
            if !endpoint.is_connected() {
                metrics::inc_rtp_dropped("subscriber_not_connected");
                continue;
            }
            let Some(outbound_payload_type) = target.payload_types.get(&inbound_payload_type).copied()
            else {
                metrics::inc_rtp_dropped("payload_map_missing");
                warn!(
                    "{}: unable to map payload type {} for {}->{} rtp ssrc {} via sender {:?}",
                    self.id,
                    inbound_payload_type,
                    endpoint_id,
                    subscriber,
                    ssrc,
                    sender_id
                );
                continue;
            };

            let forwarded_rtp = RtcLobby::translate_rtp_for_subscriber(
                rtp_packet,
                outbound_payload_type,
                &target.extension_rewrites,
                target.subscriber_mid.as_deref(),
            );

            trace!(
                "{}: {}->{} forward rtp ssrc {} pt {} -> {} via sender {:?}",
                self.id,
                endpoint_id,
                subscriber,
                ssrc,
                inbound_payload_type,
                outbound_payload_type,
                sender_id
            );
            let write_result = endpoint
                .rtp_sender(sender_id)
                .ok_or(Error::ErrRTPSenderNotExisted)
                .and_then(|mut sender| sender.write_rtp(forwarded_rtp));
            if let Err(err) = write_result {
                metrics::inc_rtp_dropped("write_error");
                warn!(
                    "{}: {}->{} forward rtp ssrc {} err: {}",
                    self.id, endpoint_id, subscriber, ssrc, err
                );
            } else {
                metrics::inc_rtp_forwarded(rtp_packet.payload.len());
            }
        }

        if let Some(started_at) = forward_started_at {
            metrics::observe_rtp_forward_delay(
                media,
                subscriber_count,
                started_at.elapsed().as_secs_f64(),
            );
        }
    }

    /// Relay subscriber keyframe requests (PLI/FIR) upstream to the publisher endpoint.
    ///
    /// Normal RTCP stays inside the endpoint's own WebRTC leg and is handled by the default
    /// interceptor chain. The lobby only sees PLI/FIR because `RtcpForwarderInterceptor`
    /// deliberately surfaces those requests to `poll_read()`.
    fn forward_rtcp(&mut self, endpoint_id: RtcEndpointId, rtcp_packets: &[Box<dyn Packet>]) {
        for packet in rtcp_packets {
            metrics::inc_rtcp_in(classify_rtcp_packet(packet.as_ref()));
        }
        let route = rtcp_packets
            .iter()
            .flat_map(|packet| packet.destination_ssrc())
            .find_map(|ssrc| {
                self.forward
                    .route_by_ssrc(ssrc)
                    .map(|(key, _subscribers)| (ssrc, key.publish_endpoint))
            });
        let Some((ssrc, publisher_id)) = route else {
            metrics::inc_rtcp_dropped("no_route");
            trace!(
                "{}: no forward binding for keyframe rtcp from {}",
                self.id,
                endpoint_id
            );
            return;
        };
        if publisher_id == endpoint_id {
            metrics::inc_rtcp_dropped("from_publisher");
            trace!(
                "{}: keyframe rtcp from publisher {} for its own ssrc {} ignored",
                self.id,
                endpoint_id,
                ssrc
            );
            return;
        }
        self.relay_keyframe_request(endpoint_id, publisher_id, ssrc, rtcp_packets);
    }

    /// Relay a subscriber's keyframe requests (PLI/FIR) about `ssrc` upstream to `publisher_id`,
    /// so the publisher's encoder emits a keyframe; without this a subscriber that renegotiates or
    /// drops a frame freezes until the publisher's next natural keyframe. RR/NACK are left to the
    /// SFU's per-leg interceptors. Non-PLI/FIR feedback (and feedback for a departed publisher) is
    /// dropped.
    fn relay_keyframe_request(
        &mut self,
        subscriber_id: RtcEndpointId,
        publisher_id: RtcEndpointId,
        ssrc: u32,
        rtcp_packets: &[Box<dyn Packet>],
    ) {
        let keyframe_requests: Vec<Box<dyn Packet>> = rtcp_packets
            .iter()
            .filter(|packet| {
                let any = packet.as_any();
                any.is::<PictureLossIndication>() || any.is::<FullIntraRequest>()
            })
            .map(|packet| packet.cloned())
            .collect();
        if keyframe_requests.is_empty() {
            metrics::inc_rtcp_keyframe_request("ignored_no_pli_fir");
            trace!(
                "{}: rtcp from subscriber {} about publisher {} ssrc {} carries no PLI/FIR — ignored",
                self.id, subscriber_id, publisher_id, ssrc
            );
            return;
        }
        trace!(
            "{}: subscriber {} -> publisher {} keyframe request ({} PLI/FIR) for ssrc {}",
            self.id,
            subscriber_id,
            publisher_id,
            keyframe_requests.len(),
            ssrc
        );
        let Some(publisher) = self.endpoints.get_mut(&publisher_id) else {
            metrics::inc_rtcp_keyframe_request("dropped_missing_publisher");
            trace!(
                "{}: publisher {} no longer in lobby — keyframe request for ssrc {} dropped",
                self.id,
                publisher_id,
                ssrc
            );
            return;
        };
        if let Err(err) = publisher.request_keyframe(ssrc, keyframe_requests) {
            metrics::inc_rtcp_keyframe_request("write_error");
            warn!(
                "{}: failed to forward keyframe request to publisher {} for ssrc {}: {}",
                self.id, publisher_id, ssrc, err
            );
        } else {
            metrics::inc_rtcp_keyframe_request("forwarded");
        }
    }

    fn handle_data_channel_message(
        &mut self,
        endpoint_id: RtcEndpointId,
        channel_id: rtc::data_channel::RTCDataChannelId,
        data: rtc::data_channel::RTCDataChannelMessage,
    ) -> Result<(), Error> {
        let source_endpoint = match self.endpoints.get(&endpoint_id) {
            Some(endpoint) => endpoint.id().clone(),
            None => {
                warn!(
                    "{}: data channel message from unknown endpoint {}",
                    self.id, endpoint_id
                );
                return Ok(());
            }
        };
        self.control
            .register_data_channel(&source_endpoint, channel_id)?
            .into_iter()
            .try_for_each(|send| self.send_control_payload(send))?;

        let message = parse_control_message(data.data.as_ref())?;
        match message {
            ControlMessage::Answer { request_id, sdp } => {
                self.handle_control_answer(&source_endpoint, request_id, sdp)
            }
            ControlMessage::Metadata(metadata) => {
                self.handle_control_metadata(&source_endpoint, metadata)
            }
        }
    }

    fn handle_control_answer(
        &mut self,
        source_endpoint: &EndpointId,
        request_id: u64,
        sdp: RTCSessionDescription,
    ) -> Result<(), Error> {
        if self
            .control
            .is_stale_answer(source_endpoint.peer_id(), request_id)
        {
            debug!(
                "{}: ignoring stale subscription answer request_id={} peer={}",
                self.id,
                request_id,
                source_endpoint.peer_id()
            );
            return Ok(());
        }

        let Some(subscription_id) = self
            .control
            .subscription_endpoint_for_peer(source_endpoint.peer_id())
        else {
            warn!(
                "{}: answer from {} has no subscription endpoint",
                self.id, source_endpoint
            );
            return Ok(());
        };
        let Some(subscription_endpoint) = self.endpoints.get(&subscription_id) else {
            warn!(
                "{}: answer from {} targets missing subscription endpoint {}",
                self.id, source_endpoint, subscription_id
            );
            return Ok(());
        };

        self.handle_event(SFUEvent::SessionDescription {
            request_id,
            rtc_lobby_id: self.id,
            endpoint_id: subscription_endpoint.id().clone(),
            sdp,
        })
    }

    fn handle_control_metadata(
        &mut self,
        source_endpoint: &EndpointId,
        metadata: ControlMetadata,
    ) -> Result<(), Error> {
        match &metadata {
            ControlMetadata::Mute { mid, mute } => {
                let key = ForwardKey {
                    publisher_peer: source_endpoint.peer_id().clone(),
                    publish_endpoint: source_endpoint.rtc_id(),
                    mid: mid.clone(),
                };
                if let Some(track) = self.published_tracks.get_mut(&key) {
                    track.muted = *mute;
                } else {
                    trace!(
                        "{}: metadata for unknown published track {}:{}",
                        self.id,
                        source_endpoint.rtc_id(),
                        mid
                    );
                }
            }
        }
        self.broadcast_metadata(source_endpoint, &metadata)
    }

    fn broadcast_metadata(
        &mut self,
        source_endpoint: &EndpointId,
        metadata: &ControlMetadata,
    ) -> Result<(), Error> {
        let subscribers: Vec<EndpointId> = self
            .subscribers
            .iter()
            .filter_map(|id| self.endpoints.get(id).map(|endpoint| endpoint.id().clone()))
            .filter(|endpoint| endpoint.peer_id() != source_endpoint.peer_id())
            .collect();

        for subscriber in subscribers {
            self.enqueue_control_message(
                subscriber.peer_id(),
                ControlOutMessage::Metadata(metadata.clone()),
            )?;
        }
        Ok(())
    }

    fn send_subscription_offer(
        &mut self,
        request_id: u64,
        endpoint_id: &EndpointId,
        sdp: &RTCSessionDescription,
    ) -> Result<(), Error> {
        if endpoint_id.kind() != EndpointKind::Subscribe || sdp.sdp_type != RTCSdpType::Offer {
            return Ok(());
        }

        debug!(
            "{}: sending subscribe renegotiation offer request_id={} peer={} endpoint={}",
            self.id,
            request_id,
            endpoint_id.peer_id(),
            endpoint_id
        );
        self.enqueue_control_message(
            endpoint_id.peer_id(),
            ControlOutMessage::Offer {
                request_id,
                sdp: sdp.clone(),
            },
        )
    }

    fn enqueue_control_message(
        &mut self,
        peer_id: &crate::sfu::peer::PeerId,
        message: ControlOutMessage,
    ) -> Result<(), Error> {
        let sends = self.control.enqueue_for_peer(peer_id, message)?;
        if sends.is_empty() {
            debug!("{}: queued control message for peer {}", self.id, peer_id);
        }
        sends
            .into_iter()
            .try_for_each(|send| self.send_control_payload(send))
    }

    fn send_control_payload(&mut self, send: ControlSend) -> Result<(), Error> {
        let Some(endpoint) = self.endpoints.get_mut(&send.endpoint_id) else {
            warn!(
                "{}: control channel endpoint {} is missing",
                self.id, send.endpoint_id
            );
            return Ok(());
        };

        debug!(
            "{}: sending control payload bytes={} endpoint={} channel={:?}",
            self.id,
            send.payload.len(),
            send.endpoint_id,
            send.channel_id
        );
        endpoint.send_data_channel_message(send.channel_id, send.payload)
    }
}

impl Protocol<TaggedBytesMut, Infallible, SFUEvent> for RtcLobby {
    type Rout = Infallible;
    type Wout = TaggedBytesMut;
    type Eout = SFUEvent;
    type Error = Error;
    type Time = Instant;

    fn handle_read(&mut self, msg: TaggedBytesMut) -> Result<(), Self::Error> {
        if let Some((rtc_lobby_id, endpoint_id)) = self.demuxer.demux(&msg) {
            if rtc_lobby_id != self.id {
                warn!(
                    "Invalid lobby {}'s message routed to lobby {}",
                    rtc_lobby_id, self.id
                );
                return Err(Error::Other(format!(
                    "Invalid lobby {}'s message routed to lobby {}",
                    self.id, self.id
                )));
            }

            if let Some(endpoint) = self.endpoints.get_mut(&endpoint_id) {
                endpoint.handle_read(msg)?;
            } else {
                warn!("Received message for unknown endpoint {}", endpoint_id);
            }
        } else {
            warn!(
                "unroutable message from {} to {}",
                msg.transport.peer_addr, msg.transport.local_addr
            );
        }
        Ok(())
    }

    fn poll_read(&mut self) -> Option<Self::Rout> {
        let mut forwardings: HashMap<RtcEndpointId, VecDeque<RTCMessage>> = HashMap::new();
        for (endpoint_id, endpoint) in &mut self.endpoints {
            while let Some(msg) = endpoint.poll_read() {
                forwardings.entry(*endpoint_id).or_default().push_back(msg);
            }
        }

        // Selective forwarding: resolve each packet's SSRC through the forward table to
        // the per-subscriber senders it fans out to. Packets whose SSRC is not bound yet
        // (first packets of a bare-m-line/simulcast publish, racing OnTrack) are dropped
        // quietly — the binding lands in this same drive iteration via poll_event.
        for (endpoint_id, mut reads) in forwardings.drain() {
            while let Some(msg) = reads.pop_front() {
                match msg {
                    RTCMessage::DataChannelMessage(data_channel_id, data) => {
                        if let Err(err) =
                            self.handle_data_channel_message(endpoint_id, data_channel_id, data)
                        {
                            warn!(
                                "{}: failed to handle data channel message from {}: {}",
                                self.id, endpoint_id, err
                            );
                        }
                    }
                    RTCMessage::RtpPacket(_, rtp_packet) => {
                        self.forward_rtp(endpoint_id, &rtp_packet)
                    }
                    RTCMessage::RtcpPacket(_, rtcp_packets) => {
                        self.forward_rtcp(endpoint_id, &rtcp_packets)
                    }
                }
            }
        }

        None
    }

    fn handle_write(&mut self, _msg: Infallible) -> Result<(), Self::Error> {
        match _msg {}
    }

    fn poll_write(&mut self) -> Option<Self::Wout> {
        for endpoint in self.endpoints.values_mut() {
            while let Some(msg) = endpoint.poll_write() {
                self.writes.push_back(msg);
            }
        }

        self.writes.pop_front()
    }

    fn handle_event(&mut self, evt: SFUEvent) -> Result<(), Self::Error> {
        let rtc_lobby_id = if let Some(rtc_lobby_id) = evt.rtc_lobby_id() {
            if rtc_lobby_id != self.id {
                return Err(Error::Other(format!("invalid lobby id: {}", rtc_lobby_id)));
            }
            rtc_lobby_id
        } else {
            return Err(Error::Other("empty lobby id".to_string()));
        };

        if let Some(endpoint_id) = evt.endpoint_id().cloned() {
            let rtc_endpoint_id = endpoint_id.rtc_id();
            // Join, Leave, and applying remote description can all
            // change the lobby's publish state, so reconcile the forwarding graph after.
            let mut needs_reconcile = false;
            let mut remove_endpoint = false;
            if let Some(endpoint) = self.endpoints.get_mut(&rtc_endpoint_id) {
                if let SFUEvent::Leave { .. } = &evt {
                    endpoint.close()?;
                    remove_endpoint = true;
                    needs_reconcile = true;
                } else if let SFUEvent::CreateOffer { request_id, .. } = &evt {
                    if let Some(endpoint) = self.endpoints.get_mut(&rtc_endpoint_id) {
                        endpoint.ensure_bootstrap_data_channel("whep")?;
                    }
                    self.reconcile();
                    if let Some(endpoint) = self.endpoints.get_mut(&rtc_endpoint_id) {
                        endpoint.create_offer(*request_id)?;
                    }
                    needs_reconcile = false;
                } else {
                    needs_reconcile = matches!(evt, SFUEvent::SessionDescription { .. });
                    endpoint.handle_event(RtcEndpointEvent::SFUEvent(evt))?;
                }
            } else if let SFUEvent::Join { .. } = &evt {
                let endpoint_kind = endpoint_id.kind();
                self.control.register_endpoint(&endpoint_id);
                let endpoint = self.build_endpoint(endpoint_id.clone(), rtc_lobby_id)?;
                match endpoint_kind {
                    EndpointKind::Publish => {
                        self.publishers.insert(rtc_endpoint_id);
                    }
                    EndpointKind::Subscribe => {
                        self.subscribers.insert(rtc_endpoint_id);
                    }
                }
                self.endpoints.insert(rtc_endpoint_id, endpoint);
                needs_reconcile = false;
            }

            if remove_endpoint {
                self.endpoints.remove(&rtc_endpoint_id);
                self.publishers.remove(&rtc_endpoint_id);
                self.subscribers.remove(&rtc_endpoint_id);
                self.control.unregister_endpoint(&endpoint_id);
            }

            if needs_reconcile {
                self.reconcile();
            }
        } else if let SFUEvent::Err {
            request_id, reason, ..
        } = evt
        {
            warn!(
                "{}:{} receives err due to {}",
                request_id, rtc_lobby_id, reason
            );
        } else if let SFUEvent::Ok { request_id, .. } = evt {
            warn!("{}:{} receives ok", request_id, rtc_lobby_id,);
        }

        Ok(())
    }

    fn poll_event(&mut self) -> Option<Self::Eout> {
        let mut subscription_offers = Vec::new();
        let mut control_sends = Vec::new();
        for (endpoint_id, endpoint) in &mut self.endpoints {
            while let Some(event) = endpoint.poll_event() {
                match event {
                    RtcEndpointEvent::SFUEvent(evt) => {
                        if let SFUEvent::SessionDescription {
                            request_id,
                            endpoint_id,
                            sdp,
                            ..
                        } = &evt
                        {
                            if endpoint_id.kind() == EndpointKind::Subscribe
                                && sdp.sdp_type == RTCSdpType::Offer
                            {
                                subscription_offers.push((
                                    *request_id,
                                    endpoint_id.clone(),
                                    sdp.clone(),
                                ));
                            }
                        }
                        self.events.push_back(evt);
                    }
                    RtcEndpointEvent::PeerConnectionEvent(RTCPeerConnectionEvent::OnDataChannel(
                        RTCDataChannelEvent::OnOpen(channel_id),
                    )) => {
                        let endpoint_identity = endpoint.id().clone();
                        let label = endpoint.data_channel_label(channel_id);
                        debug!(
                            "{}: data channel open endpoint={} label={:?} channel={:?}",
                            self.id,
                            endpoint_identity,
                            label,
                            channel_id
                        );
                        if label.as_deref() == Some("whip") {
                            match self.control.register_data_channel(&endpoint_identity, channel_id)
                            {
                                Ok(sends) => {
                                    control_sends.extend(sends);
                                }
                                Err(err) => warn!(
                                    "{}: failed to register control channel for peer {}: {}",
                                    self.id,
                                    endpoint_identity.peer_id(),
                                    err
                                ),
                            }
                            debug!(
                                "{}: registered control channel peer={} endpoint={} channel={:?}",
                                self.id,
                                endpoint_identity.peer_id(),
                                endpoint_identity.rtc_id(),
                                channel_id
                            );
                        }
                    }
                    RtcEndpointEvent::PeerConnectionEvent(RTCPeerConnectionEvent::OnTrack(
                        RTCTrackEvent::OnOpen(init),
                    )) => {
                        // Packet-time SSRC binding: the definitive wire SSRC for publish
                        // streams the SDP couldn't name up front (bare m-line without
                        // `a=ssrc`, or RID-based simulcast — one OnOpen per layer, all
                        // binding to the same {publisher, mid} key).
                        if let Some(mid) = endpoint.transceiver_mid(init.receiver_id) {
                            if self.publishers.contains(endpoint_id) {
                                self.forward.bind_ssrc(
                                    init.ssrc,
                                    ForwardKey {
                                        publisher_peer: endpoint.id().peer_id().clone(),
                                        publish_endpoint: *endpoint_id,
                                        mid,
                                    },
                                );
                            }
                        } else {
                            warn!(
                                "{}: OnTrack(OnOpen) ssrc {} from {} has no mid — not bound",
                                self.id, init.ssrc, endpoint_id
                            );
                        }
                    }
                    RtcEndpointEvent::PeerConnectionEvent(_) => {
                        //TODO: remaining peer connection events
                    }
                }
            }
        }
        for send in control_sends {
            if let Err(err) = self.send_control_payload(send) {
                warn!("{}: failed to flush queued control payload: {}", self.id, err);
            }
        }
        for (request_id, endpoint_id, sdp) in subscription_offers {
            if let Err(err) = self.send_subscription_offer(request_id, &endpoint_id, &sdp) {
                warn!(
                    "{}: failed to send subscription offer over control channel to {}: {}",
                    self.id, endpoint_id, err
                );
            }
        }
        self.events.pop_front()
    }

    fn handle_timeout(&mut self, now: Self::Time) -> Result<(), Self::Error> {
        let mut errs: Vec<Error> = vec![];
        for endpoint in self.endpoints.values_mut() {
            if let Err(err) = endpoint.handle_timeout(now) {
                errs.push(err);
            }
        }
        flatten_errs(errs)
    }

    fn poll_timeout(&mut self) -> Option<Self::Time> {
        let mut eto: Option<Instant> = None;
        for endpoint in self.endpoints.values_mut() {
            if let Some(next) = endpoint.poll_timeout() {
                eto = Some(eto.map_or(next, |curr| std::cmp::min(curr, next)));
            }
        }
        eto
    }

    fn close(&mut self) -> Result<(), Self::Error> {
        self.endpoints.clear();
        self.publishers.clear();
        self.subscribers.clear();
        self.published_tracks.clear();
        self.forward.clear();
        self.writes.clear();
        self.events.clear();
        Ok(())
    }
}

fn classify_rtcp_packet(packet: &dyn Packet) -> &'static str {
    let any = packet.as_any();
    if any.is::<PictureLossIndication>() {
        return "pli";
    }
    if any.is::<FullIntraRequest>() {
        return "fir";
    }
    if any.is::<TransportLayerNack>() {
        return "nack";
    }
    if any.is::<ReceiverReport>() {
        return "receiver_report";
    }
    if any.is::<SenderReport>() {
        return "sender_report";
    }

    let header = packet.header();
    if header.packet_type == PacketType::TransportSpecificFeedback && header.count == FORMAT_CCFB {
        return "ccfb";
    }

    "unknown"
}

fn media_kind_label(kind: RtpCodecKind) -> &'static str {
    match kind {
        RtpCodecKind::Audio => "audio",
        RtpCodecKind::Video => "video",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A lobby id whose base64 contains **both** ufrag separators (`dzax4qN2/q/IfVWKlhcc+w`),
    /// which is the case the `{lobby}/{peer}+{ufrag}` scheme has to survive.
    const ADVERSARIAL: &str = "7736b1e2-a376-feaf-c87d-558a96171cfb";

    fn uuid(text: &str) -> RtcLobbyId {
        RtcLobbyId::parse_str(text).expect("test uuid")
    }

    #[test]
    fn round_trips_every_uuid_shape() {
        let ids = [
            RtcLobbyId::nil(),
            RtcLobbyId::max(),
            RtcLobbyId::from_u128(42), // the numeric lobby ids the chat example widens
            RtcLobbyId::from_u128(u64::MAX as u128),
            uuid(ADVERSARIAL),
            uuid("a2b40a86-cc37-fc72-37d5-4940fc52c41e"), // two slashes
            RtcLobbyId::from_bytes([0x7f; 16]),
            RtcLobbyId::from_bytes([
                0x00, 0xff, 0x10, 0xef, 0x20, 0xdf, 0x30, 0xcf, 0x40, 0xbf, 0x50, 0xaf, 0x60, 0x9f,
                0x70, 0x8f,
            ]),
        ];
        for id in ids {
            let token = encode_rtc_lobby_id(&id);
            assert_eq!(token.len(), 22, "16 bytes is always 22 unpadded characters");
            assert_eq!(
                decode_rtc_lobby_id(&token),
                Some(id),
                "round trip failed for {id}"
            );
        }
    }

    #[test]
    fn decodes_only_sixteen_byte_payloads() {
        let token = encode_rtc_lobby_id(&uuid(ADVERSARIAL));
        for invalid in [
            "",                         // empty
            "AAAA",                     // 3 bytes
            &token[..token.len() - 1],  // 21 characters: not a whole 16 bytes
            &format!("{token}AA"),      // 24 characters: 18 bytes
            &format!("{token}{token}"), // 32 bytes
        ] {
            assert_eq!(decode_rtc_lobby_id(invalid), None, "accepted {invalid:?}");
        }
    }

    #[test]
    fn decodes_only_the_canonical_spelling() {
        let id = uuid(ADVERSARIAL);
        let token = encode_rtc_lobby_id(&id);

        // 128 bits is not a multiple of 6, so the final character carries 2 significant
        // bits and 4 that must be zero. If those were ignored, sixteen different strings
        // would decode to this same UUID — and since rtc_lobbies are keyed by the decoded value,
        // a peer could reach one lobby under a spelling the server never issued.
        let alphabet: Vec<char> =
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
                .chars()
                .collect();
        let last = alphabet
            .iter()
            .position(|c| *c == token.chars().last().unwrap())
            .expect("final character is in the alphabet");
        let mut twins = 0;
        for bump in 1..16 {
            let twin: String = token[..token.len() - 1]
                .chars()
                .chain(std::iter::once(alphabet[last + bump]))
                .collect();
            assert_ne!(twin, token);
            assert_eq!(decode_rtc_lobby_id(&twin), None, "accepted twin {twin:?}");
            twins += 1;
        }
        assert_eq!(
            twins, 15,
            "every non-canonical trailing-bit spelling was tested"
        );
    }

    #[test]
    fn decodes_only_the_standard_alphabet() {
        // The lobby id shares the ufrag with `+` and `/` separators, so this codec uses the
        // standard alphabet rather than base64url. A URL-safe spelling of the same bytes
        // must not decode, or two encodings would name the same lobby.
        let id = uuid(ADVERSARIAL);
        let url_safe = encode_rtc_lobby_id(&id).replace('+', "-").replace('/', "_");
        assert_ne!(url_safe, encode_rtc_lobby_id(&id));
        assert_eq!(decode_rtc_lobby_id(&url_safe), None);

        // Padding is not part of the encoding either.
        assert_eq!(
            decode_rtc_lobby_id(&format!("{}==", encode_rtc_lobby_id(&id))),
            None
        );
        assert_eq!(decode_rtc_lobby_id("not base64!!!!!!!!!!!!"), None);
    }

    #[test]
    fn local_ufrag_round_trips_through_both_separators() {
        // Standard base64 can contain `+` and `/`, which are also this scheme's own
        // separators, so a lobby id carrying both is the case that has to survive.
        for id in [
            uuid(ADVERSARIAL),                            // base64 holds both `+` and `/`
            uuid("a2b40a86-cc37-fc72-37d5-4940fc52c41e"), // two slashes
            RtcLobbyId::nil(),
            RtcLobbyId::max(),
            RtcLobbyId::from_u128(42),
        ] {
            for endpoint_id in [0 as RtcEndpointId, 1, 537_821_252, RtcEndpointId::MAX] {
                let local_ufrag = encode_local_ufrag(&id, endpoint_id);
                assert_eq!(
                    decode_local_ufrag(&local_ufrag),
                    Some((id, endpoint_id)),
                    "round trip failed for {local_ufrag}"
                );

                // RFC 8839: ufrag = 4*256(ALPHA / DIGIT / "+" / "/").
                assert!((4..=256).contains(&local_ufrag.len()));
                assert!(
                    local_ufrag
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/'),
                    "ufrag has a character ICE does not allow: {local_ufrag}"
                );
            }
        }
        let token = encode_rtc_lobby_id(&uuid(ADVERSARIAL));
        assert!(
            token.contains('+') && token.contains('/'),
            "the fixture no longer exercises both separators: {token}"
        );
    }

    #[test]
    fn local_ufrag_decoding_rejects_malformed_input() {
        let token = encode_rtc_lobby_id(&uuid(ADVERSARIAL));
        for invalid in [
            String::new(),                    // empty
            token.clone(),                    // no separators at all
            format!("{token}/7"),             // missing the random suffix
            format!("{token}+abc"),           // missing the peer id
            format!("{token}/notdigits+abc"), // peer id is not a number
            format!("{token}/-1+abc"),        // negative peer id
            "zzzz/7+abc".to_string(),         // lobby id is not 16 bytes
        ] {
            assert_eq!(decode_local_ufrag(&invalid), None, "accepted {invalid:?}");
        }
    }

    #[test]
    fn local_ufrag_is_unique_per_peer() {
        // The random suffix is what stops two endpoints of one lobby from sharing ICE
        // credentials, so the same inputs must not produce the same ufrag twice.
        let id = uuid(ADVERSARIAL);
        let first = encode_local_ufrag(&id, 7);
        let second = encode_local_ufrag(&id, 7);
        assert_ne!(first, second);
        assert_eq!(decode_local_ufrag(&first), decode_local_ufrag(&second));
    }
}
