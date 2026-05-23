#[cfg(test)]
mod tests {
    use crate::sfu::media::router::Router;
    use crate::sfu::media::{Media, MediaId};
    use crate::sfu::peer::PeerId;
    use std::collections::HashMap;
    use tokio::sync::broadcast;
    // --- Helper -------------------------------------------------------------

    fn peer(id: &str) -> PeerId {
        PeerId::new(id.to_string())
    }

    fn media(id: &str, peer_id: &PeerId, mid: &str) -> Media {
        let (rtp_tx, _dummy_rx) = broadcast::channel(1);
        Media {
            id: MediaId(id.to_string()),
            peer_id: peer_id.clone(),
            mid: mid.to_string(),
            src_track_id: "track".into(),
            src_stream_id: "stream".into(),
            capability: Default::default(),
            kind: Default::default(),
            rtp_tx,
            stopped: Default::default(),
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

    // --- Tests --------------------------------------------------------------

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
}
