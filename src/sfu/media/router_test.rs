#[cfg(test)]
mod tests {
    use crate::sfu::media::router::Router;
    use crate::sfu::media::{ForwardTarget, Media, MediaId, MediaPurpose, RtpCodecType};
    use crate::sfu::peer::PeerId;
    use crate::sfu::rtc::header_extension_translator::RtpHeaderExtensionTranslator;
    use rtc::media_stream::MediaStreamTrack;
    use rtc::rtp_transceiver::rtp_sender::RtpCodecKind;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::broadcast;
    use webrtc::media_stream::track_local::static_rtp::TrackLocalStaticRTP;
    // --- Helper -------------------------------------------------------------

    fn peer(id: &str) -> PeerId {
        PeerId::new(id.to_string())
    }

    fn media(id: &str, peer_id: &PeerId, mid: &str) -> Media {
        let (relay_rtp_tx, _relay_rtp_rx) = broadcast::channel(1);
        Media {
            id: MediaId(id.to_string()),
            peer_id: peer_id.clone(),
            mid: mid.to_string(),
            src_track_id: "track".into(),
            src_stream_id: "stream".into(),
            header_extensions: HashMap::new(),
            capability: Default::default(),
            kind: Default::default(),
            ssrc: Default::default(),
            source_track: None,
            targets: Default::default(),
            relay_rtp_tx,
            purpose: Default::default(),
            muted: false,
            info: "info".into(),
            payload_type: Default::default(),
        }
    }

    fn router_with_medias() -> Router {
        let peer_a = peer("peer-a");
        let peer_b = peer("peer-b");

        let mut medias = HashMap::new();

        medias.insert(MediaId("m1".into()), media("m1", &peer_a, "video-0"));
        medias.insert(MediaId("m2".into()), media("m2", &peer_a, "audio-0"));
        medias.insert(MediaId("m3".into()), media("m3", &peer_b, "video-0"));

        Router { medias }
    }

    fn target(peer_id: PeerId) -> ForwardTarget {
        let track = Arc::new(TrackLocalStaticRTP::new(MediaStreamTrack::new(
            "stream".into(),
            "track".into(),
            "test".into(),
            RtpCodecKind::Video,
            vec![],
        )));
        ForwardTarget::new(
            peer_id,
            track,
            RtpHeaderExtensionTranslator::new(&HashMap::new(), &[], "0".into()),
        )
    }

    // --- Tests --------------------------------------------------------------

    #[test]
    fn relay_uses_only_host_tracks_and_survives_guest_removal() {
        let host = peer("host");
        let guest = peer("guest-first");
        let mut router = Router::new();
        for (owner, prefix) in [(&guest, "guest"), (&host, "host")] {
            for (kind, suffix) in [(RtpCodecType::Audio, "audio"), (RtpCodecType::Video, "video")] {
                let mut track = media(&format!("{prefix}-{suffix}"), owner, suffix);
                track.kind = kind;
                track.purpose = MediaPurpose::STREAM;
                router.medias.insert(track.id.clone(), track);
            }
        }
        let selected = router.get_media_stream(&host);
        assert_eq!(selected.audio.unwrap().peer_id, host);
        assert_eq!(selected.video.unwrap().peer_id, host);
        router.remove_medias_of_peer(&guest);
        let selected = router.get_media_stream(&host);
        assert_eq!(selected.audio.unwrap().peer_id, host);
        assert_eq!(selected.video.unwrap().peer_id, host);
        router.remove_medias_of_peer(&host);
        let selected = router.get_media_stream(&host);
        assert!(selected.audio.is_none());
        assert!(selected.video.is_none());
    }

    #[test]
    fn relay_does_not_fall_back_to_guest_tracks_when_host_is_missing() {
        let mut router = Router::new();
        let mut track = media("guest-video", &peer("guest"), "0");
        track.kind = RtpCodecType::Video;
        track.purpose = MediaPurpose::STREAM;
        router.medias.insert(track.id.clone(), track);
        let selected = router.get_media_stream(&peer("host"));
        assert!(selected.audio.is_none());
        assert!(selected.video.is_none());
    }

    #[test]
    fn test_media_lookup_by_id() {
        let router = router_with_medias();

        let id = MediaId("m1".into());
        let media = router.media(&id).expect("media should exist");

        assert_eq!(media.mid, "video-0");
    }

    #[test]
    fn test_get_media_of_peer_by_mid() {
        let mut router = router_with_medias();
        let peer_a = peer("peer-a");

        let media = router
            .get_media_of_peer_by_mid(&peer_a, "audio-0")
            .expect("media should exist");

        assert_eq!(media.mid, "audio-0");
        assert_eq!(media.peer_id, peer_a);
    }

    #[test]
    fn test_get_media_of_peer_by_mid_and_mute() {
        let mut router = router_with_medias();
        let peer_a = peer("peer-a");
        {
            let media = router
                .get_media_of_peer_by_mid(&peer_a, "audio-0")
                .expect("media should exist");

            media.set_mut(true);
        }

        let media_again = router
            .get_media_of_peer_by_mid(&peer_a, "audio-0")
            .expect("media should exist");

        assert_eq!(media_again.mid, "audio-0");
        assert_eq!(media_again.peer_id, peer_a);
        assert!(media_again.muted);
    }

    #[test]
    fn test_get_media_of_peer_by_mid_not_found() {
        let mut router = router_with_medias();
        let peer_a = peer("peer-a");

        let media = router.get_media_of_peer_by_mid(&peer_a, "does-not-exist");
        assert!(media.is_none());
    }

    #[test]
    fn test_get_medias_without_peer() {
        let router = router_with_medias();
        let peer_a = peer("peer-a");

        let medias = router.get_medias_without_peer(&peer_a);

        assert_eq!(medias.len(), 1);
        assert_eq!(medias[0].peer_id, peer("peer-b"));
    }

    #[test]
    fn test_remove_medias_of_peer() {
        let mut router = router_with_medias();
        let peer_a = peer("peer-a");

        router.remove_medias_of_peer(&peer_a);

        // peer-a medias removed
        assert!(router.medias.values().all(|m| m.peer_id != peer_a));

        // peer-b still present
        assert_eq!(router.medias.len(), 1);
    }

    #[test]
    fn test_activate_and_remove_target() {
        let router = router_with_medias();
        let media_id = MediaId("m1".into());
        let peer_b = peer("peer-b");

        assert!(router
            .activate_target(&media_id, target(peer_b.clone()))
            .is_some());
        assert!(router
            .activate_target(&media_id, target(peer_b.clone()))
            .is_none());

        router.remove_targets_for_peer(&peer_b);

        assert!(router.activate_target(&media_id, target(peer_b)).is_some());
    }

    #[test]
    fn test_target_confirms_rtp_binding_once() {
        let target = target(peer("peer-b"));

        assert!(target.confirm_rtp_binding());
        assert!(!target.confirm_rtp_binding());
    }
}
