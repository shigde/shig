use super::demuxer::Demuxer;
use super::event::SFUEvent;
use super::lobby::{RtcLobby, RtcLobbyId};
use log::{info, warn};
use rtc::shared::error::{flatten_errs, Error};
use rtc::shared::TaggedBytesMut;
use sansio::Protocol;
use std::collections::{HashMap, VecDeque};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::time::Instant;

pub type MediaEngineId = u64;

pub struct MediaEngine {
    id: MediaEngineId,
    local_addr: SocketAddr,
    demuxer: Demuxer,
    rtc_lobbies: HashMap<RtcLobbyId, RtcLobby>,

    writes: VecDeque<TaggedBytesMut>,
    events: VecDeque<SFUEvent>,
}

impl MediaEngine {
    pub fn new(id: MediaEngineId, local_addr: SocketAddr) -> Self {
        Self {
            id,
            local_addr,

            demuxer: Default::default(),
            rtc_lobbies: Default::default(),
            writes: Default::default(),
            events: Default::default(),
        }
    }
}

impl Protocol<TaggedBytesMut, Infallible, SFUEvent> for MediaEngine {
    type Rout = Infallible;
    type Wout = TaggedBytesMut;
    type Eout = SFUEvent;
    type Error = Error;
    type Time = Instant;

    fn handle_read(&mut self, msg: TaggedBytesMut) -> Result<(), Self::Error> {
        if let Some((rtc_lobby_id, _peer_id)) = self.demuxer.demux(&msg) {
            if let Some(lobby) = self.rtc_lobbies.get_mut(&rtc_lobby_id) {
                lobby.handle_read(msg)?;
            } else {
                warn!("Received message for unknown lobby {}", rtc_lobby_id);
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
        for lobby in self.rtc_lobbies.values_mut() {
            while let Some(msg) = lobby.poll_read() {
                info!("process lobby's poll_read {:?}, should always be None", msg);
            }
        }
        None
    }

    fn handle_write(&mut self, _msg: Infallible) -> Result<(), Self::Error> {
        match _msg {}
    }

    fn poll_write(&mut self) -> Option<Self::Wout> {
        for lobby in self.rtc_lobbies.values_mut() {
            while let Some(msg) = lobby.poll_write() {
                self.writes.push_back(msg);
            }
        }
        self.writes.pop_front()
    }

    fn handle_event(&mut self, evt: SFUEvent) -> Result<(), Self::Error> {
        if let Some(rtc_lobby_id) = evt.rtc_lobby_id() {
            let mut remove_lobby = false;
            if let Some(lobby) = self.rtc_lobbies.get_mut(&rtc_lobby_id) {
                let is_leave_event = matches!(evt, SFUEvent::Leave { .. });
                lobby.handle_event(evt)?;
                if is_leave_event && lobby.is_empty() {
                    remove_lobby = true;
                }
            } else if let SFUEvent::Join { .. } = &evt {
                let mut lobby = RtcLobby::new(rtc_lobby_id, self.local_addr);
                lobby.handle_event(evt)?;
                self.rtc_lobbies.insert(rtc_lobby_id, lobby);
            }

            if remove_lobby {
                self.rtc_lobbies.remove(&rtc_lobby_id);
            }
        } else if let SFUEvent::Err {
            request_id, reason, ..
        } = evt
        {
            warn!("{} receives err due to {}", request_id, reason);
        } else if let SFUEvent::Ok { request_id, .. } = evt {
            warn!("{} receives ok", request_id);
        }

        Ok(())
    }

    fn poll_event(&mut self) -> Option<Self::Eout> {
        for lobby in self.rtc_lobbies.values_mut() {
            while let Some(event) = lobby.poll_event() {
                self.events.push_back(event);
            }
        }

        self.events.pop_front()
    }

    fn handle_timeout(&mut self, now: Self::Time) -> Result<(), Self::Error> {
        let mut errs: Vec<Error> = vec![];
        for lobby in self.rtc_lobbies.values_mut() {
            if let Err(err) = lobby.handle_timeout(now) {
                errs.push(err);
            }
        }
        flatten_errs(errs)
    }

    fn poll_timeout(&mut self) -> Option<Self::Time> {
        let mut eto: Option<Instant> = None;
        for lobby in self.rtc_lobbies.values_mut() {
            if let Some(next) = lobby.poll_timeout() {
                eto = Some(eto.map_or(next, |curr| std::cmp::min(curr, next)));
            }
        }
        eto
    }

    fn close(&mut self) -> Result<(), Self::Error> {
        self.rtc_lobbies.clear();
        self.writes.clear();
        self.events.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sfu::rtc::media::endpoint::RtcEndpointId;
    use crate::sfu::rtc::media::event::{RequestId, SFUEvent};
    use crate::sfu::rtc::media::lobby::RtcLobbyId;
    use rtc::peer_connection::configuration::media_engine::MediaEngine as RtcMediaEngine;
    use rtc::peer_connection::sdp::{RTCSdpType, RTCSessionDescription};
    use rtc::peer_connection::RTCPeerConnectionBuilder;
    use rtc::rtp_transceiver::rtp_sender::{
        RTCRtpCodingParameters, RTCRtpEncodingParameters, RtpCodecKind,
    };
    use rtc::rtp_transceiver::{RTCRtpTransceiverDirection, RTCRtpTransceiverInit};

    const LOBBY: RtcLobbyId = RtcLobbyId::from_u128(100);
    const ENDPOINT: RtcEndpointId = 200;

    /// A browser-side peer connection that publishes one video track, used only to
    /// produce a valid SDP offer to feed into the SFU.
    fn build_offer() -> RTCSessionDescription {
        build_offer_with_ssrc(111_111)
    }

    fn build_offer_with_ssrc(ssrc: u32) -> RTCSessionDescription {
        let mut media_engine = RtcMediaEngine::default();
        media_engine
            .register_default_codecs()
            .expect("default codecs should register");

        let mut offerer = RTCPeerConnectionBuilder::new()
            .with_media_engine(media_engine)
            .build()
            .expect("offerer peer connection should build");

        // Publish sendonly with an explicit SSRC, like the real browser (chat.html), so
        // the SFU answers recvonly rather than mirroring a sendrecv transceiver and
        // re-offering.
        offerer
            .add_transceiver_from_kind(
                RtpCodecKind::Video,
                Some(RTCRtpTransceiverInit {
                    direction: RTCRtpTransceiverDirection::Sendonly,
                    streams: Vec::new(),
                    send_encodings: vec![RTCRtpEncodingParameters {
                        rtp_coding_parameters: RTCRtpCodingParameters {
                            ssrc: Some(ssrc),
                            ..Default::default()
                        },
                        active: true,
                        ..Default::default()
                    }],
                }),
            )
            .expect("video transceiver should be added");

        let offer = offerer.create_offer(None).expect("offer should be created");
        assert_eq!(offer.sdp_type, RTCSdpType::Offer);
        assert!(!offer.sdp.is_empty());
        offer
    }

    /// Complete a peer's first SDP negotiation with an application-only (data channel) offer —
    /// the way a pure subscriber joins: no media is published, but the initial SDP round
    /// completes so the SFU may then re-offer forwards to it. The SFU never makes the first
    /// offer, so a subscriber must send this before it can be forwarded to. Drains (and discards)
    /// the resulting answer.
    fn negotiate_endpoint(
        worker: &mut MediaEngine,
        request_id: RequestId,
        endpoint_id: RtcEndpointId,
    ) {
        worker
            .handle_event(SFUEvent::SessionDescription {
                request_id,
                rtc_lobby_id: LOBBY,
                endpoint_id,
                sdp: build_bootstrap_offer(),
            })
            .expect("endpoint bootstrap offer should be handled");
        drain_events(worker);
    }

    /// An application-only offer: a single data channel, no media m-lines. Mirrors the bootstrap
    /// offer a pure subscriber sends to complete its first SDP negotiation.
    fn build_bootstrap_offer() -> RTCSessionDescription {
        let mut media_engine = RtcMediaEngine::default();
        media_engine
            .register_default_codecs()
            .expect("default codecs should register");
        let mut offerer = RTCPeerConnectionBuilder::new()
            .with_media_engine(media_engine)
            .build()
            .expect("offerer peer connection should build");
        offerer
            .create_data_channel("bootstrap", None)
            .expect("data channel should be created");
        let offer = offerer.create_offer(None).expect("offer should be created");
        assert_eq!(offer.sdp_type, RTCSdpType::Offer);
        offer
    }

    fn build_offer_with_extra_video_codec(
        payload_type: u8,
        codec_name: &str,
    ) -> RTCSessionDescription {
        let mut offer = build_offer();
        let mut lines: Vec<String> = offer.sdp.split("\r\n").map(str::to_owned).collect();

        let video_line = lines
            .iter_mut()
            .find(|line| line.starts_with("m=video "))
            .expect("offer should contain a video m-line");
        video_line.push_str(&format!(" {payload_type}"));

        let insert_at = lines
            .iter()
            .rposition(|line| !line.is_empty())
            .map(|idx| idx + 1)
            .unwrap_or(lines.len());
        lines.insert(
            insert_at,
            format!("a=rtpmap:{payload_type} {codec_name}/90000"),
        );

        offer.sdp = lines.join("\r\n");
        offer
    }

    fn join(worker: &mut MediaEngine, request_id: RequestId) {
        join_endpoint(worker, request_id, ENDPOINT);
    }

    fn join_endpoint(worker: &mut MediaEngine, request_id: RequestId, endpoint_id: RtcEndpointId) {
        worker
            .handle_event(SFUEvent::Join {
                request_id,
                rtc_lobby_id: LOBBY,
                endpoint_id,
                participant_id: format!("00000000-0000-4000-8000-{endpoint_id:012}"),
            })
            .expect("join should succeed");
    }

    fn drain_events(worker: &mut MediaEngine) -> Vec<SFUEvent> {
        let mut events = Vec::new();
        while let Some(event) = worker.poll_event() {
            events.push(event);
        }
        events
    }

    #[test]
    fn join_creates_lobby_and_peer() {
        let mut worker = MediaEngine::new(0, "0.0.0.0:0".parse().unwrap());
        assert!(worker.rtc_lobbies.is_empty());

        join(&mut worker, 1);

        let lobby = worker
            .rtc_lobbies
            .get(&LOBBY)
            .expect("lobby should exist after join");
        assert_eq!(lobby.id(), LOBBY);
        assert!(
            !lobby.is_empty(),
            "lobby should contain the joined endpoint"
        );
    }

    #[test]
    fn leave_removes_peer_and_reaps_empty_lobby() {
        let mut worker = MediaEngine::new(0, "0.0.0.0:0".parse().unwrap());
        join(&mut worker, 1);
        assert!(worker.rtc_lobbies.contains_key(&LOBBY));

        worker
            .handle_event(SFUEvent::Leave {
                request_id: 2,
                rtc_lobby_id: LOBBY,
                endpoint_id: ENDPOINT,
                reason: "bye".to_string(),
            })
            .expect("leave should succeed");

        // The last endpoint left, so the MediaEngine self-reaps the now-empty lobby.
        assert!(
            !worker.rtc_lobbies.contains_key(&LOBBY),
            "empty lobby should be removed after the last endpoint leaves"
        );
    }

    #[test]
    fn session_description_offer_returns_answer() {
        let mut worker = MediaEngine::new(0, "0.0.0.0:0".parse().unwrap());
        join(&mut worker, 1);

        let request_id: RequestId = 2;
        worker
            .handle_event(SFUEvent::SessionDescription {
                request_id,
                rtc_lobby_id: LOBBY,
                endpoint_id: ENDPOINT,
                sdp: build_offer(),
            })
            .expect("handling the offer should succeed");

        let event = worker
            .poll_event()
            .expect("the SFU should emit an answer for the offer");

        match event {
            SFUEvent::SessionDescription {
                request_id: got_request_id,
                rtc_lobby_id,
                endpoint_id,
                sdp,
            } => {
                assert_eq!(got_request_id, request_id);
                assert_eq!(rtc_lobby_id, LOBBY);
                assert_eq!(endpoint_id, ENDPOINT);
                assert_eq!(
                    sdp.sdp_type,
                    RTCSdpType::Answer,
                    "the SFU should answer an offer"
                );
                assert!(!sdp.sdp.is_empty(), "the answer SDP should not be empty");
            }
            other => panic!("expected a SessionDescription answer, got {:?}", other),
        }

        // Only the answer is surfaced (a lone sendonly publisher has no subscribers, so
        // reconcile adds no forwarding senders and no subscribe offer is produced).
        assert!(worker.poll_event().is_none());
    }

    /// The forwarding track carries every codec the publisher advertised (one per coding),
    /// so the subscriber's server-initiated offer advertises them all — not just the
    /// primary — letting it receive whichever codec the publisher actually sends.
    #[test]
    fn subscribe_offer_advertises_all_publisher_codecs() {
        const SUBSCRIBER_ENDPOINT: RtcEndpointId = 300;

        let mut worker = MediaEngine::new(0, "0.0.0.0:0".parse().unwrap());
        join_endpoint(&mut worker, 1, ENDPOINT);
        join_endpoint(&mut worker, 2, SUBSCRIBER_ENDPOINT);

        // The subscriber negotiates first — the SFU never makes the first offer.
        negotiate_endpoint(&mut worker, 3, SUBSCRIBER_ENDPOINT);

        worker
            .handle_event(SFUEvent::SessionDescription {
                request_id: 4,
                rtc_lobby_id: LOBBY,
                endpoint_id: ENDPOINT,
                sdp: build_offer(),
            })
            .expect("handling the publisher offer should succeed");

        let events = drain_events(&mut worker);
        let offer = events
            .iter()
            .find_map(|event| match event {
                SFUEvent::SessionDescription {
                    endpoint_id, sdp, ..
                } if *endpoint_id == SUBSCRIBER_ENDPOINT && sdp.sdp_type == RTCSdpType::Offer => {
                    Some(&sdp.sdp)
                }
                _ => None,
            })
            .expect("subscriber should receive a server-initiated offer");

        // The default video media engine registers many codecs; the forwarded m-line must
        // advertise more than the single primary one.
        let codec_count = offer.matches("a=rtpmap:").count();
        assert!(
            codec_count > 1,
            "subscribe offer should advertise all publisher codecs, got {codec_count} rtpmap(s)"
        );
    }

    #[test]
    fn publish_triggers_subscribe_offer_to_other_peer() {
        const SUBSCRIBER_ENDPOINT: RtcEndpointId = 300;

        let mut worker = MediaEngine::new(0, "0.0.0.0:0".parse().unwrap());
        join_endpoint(&mut worker, 1, ENDPOINT);
        join_endpoint(&mut worker, 2, SUBSCRIBER_ENDPOINT);

        // The subscriber completes its own first SDP negotiation first — the SFU never makes the
        // first offer, so a subscribe re-offer can only follow the endpoint's initial offer.
        negotiate_endpoint(&mut worker, 3, SUBSCRIBER_ENDPOINT);

        // ENDPOINT publishes one sendonly video track.
        worker
            .handle_event(SFUEvent::SessionDescription {
                request_id: 4,
                rtc_lobby_id: LOBBY,
                endpoint_id: ENDPOINT,
                sdp: build_offer(),
            })
            .expect("handling the publisher offer should succeed");

        let events = drain_events(&mut worker);

        // The publisher gets its answer...
        assert!(
            events.iter().any(|e| matches!(
                e,
                SFUEvent::SessionDescription { endpoint_id, sdp, .. }
                    if *endpoint_id == ENDPOINT && sdp.sdp_type == RTCSdpType::Answer
            )),
            "publisher should receive an answer, got {events:?}"
        );

        // ...and reconcile forwards the track to the subscriber, whose peer connection
        // fires OnNegotiationNeeded, producing a subscribe *offer* addressed to it.
        assert!(
            events.iter().any(|e| matches!(
                e,
                SFUEvent::SessionDescription { endpoint_id, sdp, .. }
                    if *endpoint_id == SUBSCRIBER_ENDPOINT && sdp.sdp_type == RTCSdpType::Offer
            )),
            "subscriber should receive a server-initiated offer, got {events:?}"
        );
    }

    #[test]
    fn subscribe_offer_filters_unsupported_publisher_codecs() {
        const SUBSCRIBER_ENDPOINT: RtcEndpointId = 300;
        const UNSUPPORTED_PT: u8 = 123;

        let mut worker = MediaEngine::new(0, "0.0.0.0:0".parse().unwrap());
        join_endpoint(&mut worker, 1, ENDPOINT);
        join_endpoint(&mut worker, 2, SUBSCRIBER_ENDPOINT);

        // The subscriber negotiates first — the SFU never makes the first offer.
        negotiate_endpoint(&mut worker, 3, SUBSCRIBER_ENDPOINT);

        worker
            .handle_event(SFUEvent::SessionDescription {
                request_id: 4,
                rtc_lobby_id: LOBBY,
                endpoint_id: ENDPOINT,
                sdp: build_offer_with_extra_video_codec(UNSUPPORTED_PT, "UNSUPPORTED"),
            })
            .expect("handling the publisher offer should succeed");

        let events = drain_events(&mut worker);
        let subscribe_offer = events
            .iter()
            .find_map(|event| match event {
                SFUEvent::SessionDescription {
                    endpoint_id, sdp, ..
                } if *endpoint_id == SUBSCRIBER_ENDPOINT && sdp.sdp_type == RTCSdpType::Offer => {
                    Some(&sdp.sdp)
                }
                _ => None,
            })
            .expect("subscriber should still receive a server-initiated offer");

        assert!(
            !subscribe_offer.contains(&format!("a=rtpmap:{UNSUPPORTED_PT} UNSUPPORTED/90000")),
            "subscribe offer should not advertise unsupported passthrough codecs: {subscribe_offer}"
        );
    }

    #[test]
    fn republish_same_offer_is_idempotent() {
        const SUBSCRIBER_ENDPOINT: RtcEndpointId = 300;

        let mut worker = MediaEngine::new(0, "0.0.0.0:0".parse().unwrap());
        join_endpoint(&mut worker, 1, ENDPOINT);
        join_endpoint(&mut worker, 2, SUBSCRIBER_ENDPOINT);

        // The subscriber negotiates first — the SFU never makes the first offer.
        negotiate_endpoint(&mut worker, 3, SUBSCRIBER_ENDPOINT);

        let offer = build_offer();
        worker
            .handle_event(SFUEvent::SessionDescription {
                request_id: 4,
                rtc_lobby_id: LOBBY,
                endpoint_id: ENDPOINT,
                sdp: offer.clone(),
            })
            .expect("first publish should succeed");
        let first = drain_events(&mut worker);
        let first_subscribe_offers = first
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    SFUEvent::SessionDescription { endpoint_id, sdp, .. }
                        if *endpoint_id == SUBSCRIBER_ENDPOINT && sdp.sdp_type == RTCSdpType::Offer
                )
            })
            .count();
        assert_eq!(first_subscribe_offers, 1, "first publish forwards once");

        // Re-applying the same publish offer must not add a duplicate forwarding sender,
        // so no new subscribe offer is generated (reconcile is idempotent).
        worker
            .handle_event(SFUEvent::SessionDescription {
                request_id: 5,
                rtc_lobby_id: LOBBY,
                endpoint_id: ENDPOINT,
                sdp: offer,
            })
            .expect("re-publish should succeed");
        let second = drain_events(&mut worker);
        let second_subscribe_offers = second
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    SFUEvent::SessionDescription { endpoint_id, sdp, .. }
                        if *endpoint_id == SUBSCRIBER_ENDPOINT && sdp.sdp_type == RTCSdpType::Offer
                )
            })
            .count();
        assert_eq!(
            second_subscribe_offers, 0,
            "re-publishing the same track must not re-forward, got {second:?}"
        );
    }

    #[test]
    fn subscribe_offer_after_publisher_published() {
        const SUBSCRIBER_ENDPOINT: RtcEndpointId = 300;

        let mut worker = MediaEngine::new(0, "0.0.0.0:0".parse().unwrap());
        join_endpoint(&mut worker, 1, ENDPOINT);

        // ENDPOINT publishes one sendonly video track.
        worker
            .handle_event(SFUEvent::SessionDescription {
                request_id: 2,
                rtc_lobby_id: LOBBY,
                endpoint_id: ENDPOINT,
                sdp: build_offer(),
            })
            .expect("handling the publisher offer should succeed");

        // Now SUBSCRIBER_ENDPOINT joins
        join_endpoint(&mut worker, 3, SUBSCRIBER_ENDPOINT);

        // Check that joining does not immediately trigger subscribe offer (because subscriber hasn't set remote description yet)
        let events_after_join = drain_events(&mut worker);
        let has_offer_after_join = events_after_join.iter().any(|e| {
            matches!(
                e,
                SFUEvent::SessionDescription { endpoint_id, sdp, .. }
                    if *endpoint_id == SUBSCRIBER_ENDPOINT && sdp.sdp_type == RTCSdpType::Offer
            )
        });
        assert!(
            !has_offer_after_join,
            "should not send subscribe offer immediately on Join"
        );

        // SUBSCRIBER_ENDPOINT sends bootstrap offer (SDP offer)
        let mut media_engine = RtcMediaEngine::default();
        media_engine
            .register_default_codecs()
            .expect("default codecs should register");
        let mut subscriber_pc = RTCPeerConnectionBuilder::new()
            .with_media_engine(media_engine)
            .build()
            .expect("subscriber pc should build");
        subscriber_pc
            .create_data_channel("bootstrap", None)
            .expect("create data channel");
        let subscriber_offer = subscriber_pc.create_offer(None).expect("create offer");

        worker
            .handle_event(SFUEvent::SessionDescription {
                request_id: 4,
                rtc_lobby_id: LOBBY,
                endpoint_id: SUBSCRIBER_ENDPOINT,
                sdp: subscriber_offer,
            })
            .expect("handling subscriber bootstrap offer should succeed");

        let events_after_bootstrap = drain_events(&mut worker);
        // SUBSCRIBER_ENDPOINT should receive a subscribe offer (re-offer)
        assert!(
            events_after_bootstrap.iter().any(|e| matches!(
                e,
                SFUEvent::SessionDescription { endpoint_id, sdp, .. }
                    if *endpoint_id == SUBSCRIBER_ENDPOINT && sdp.sdp_type == RTCSdpType::Offer
            )),
            "subscriber should receive a server-initiated offer, got {events_after_bootstrap:?}"
        );
    }
}
