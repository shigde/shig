use crate::sfu::media::{Media, MediaId, MediaPurpose};
use crate::sfu::peer::PeerId;
use crate::sfu::relay::RelayMediaStream;
use std::collections::HashMap;
use webrtc::rtp_transceiver::rtp_codec::RTPCodecType;

pub struct Router {
    pub medias: HashMap<MediaId, Media>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            medias: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn media(&self, id: &MediaId) -> Option<&Media> {
        self.medias.get(id)
    }

    #[allow(dead_code)]
    pub fn remove_medias_of_peer(&mut self, peer_id: &PeerId) {
        self.medias.retain(|_, media| &media.peer_id != peer_id);
    }

    /// Returns all media *not* belonging to this peer.
    pub fn get_medias_without_peer(&self, peer_id: &PeerId) -> Vec<Media> {
        self.medias
            .values()
            .filter(|&media| &media.peer_id != peer_id)
            .cloned()
            .collect()
    }

    /// Returns all media for a specific peer.
    pub fn get_medias_of_peer(&self, peer_id: &PeerId) -> Vec<Media> {
        self.medias
            .values()
            .filter(|&media| &media.peer_id == peer_id)
            .cloned()
            .collect()
    }

    pub fn get_media_of_peer_by_mid(&mut self, peer_id: &PeerId, mid: &str) -> Option<&mut Media> {
        self.medias
            .values_mut()
            .find(|m| &m.peer_id == peer_id && m.mid == mid)
    }

    pub fn get_media_stream(&self) -> RelayMediaStream {
        let mut stream = RelayMediaStream {
            audio: None,
            video: None,
        };

        for media in self.medias.values() {
            if media.purpose == MediaPurpose::STREAM {
                match media.kind {
                    RTPCodecType::Audio => stream.audio = Some(media.clone()),
                    RTPCodecType::Video => stream.video = Some(media.clone()),
                    _ => continue,
                }
            }
        }
        stream
    }
}
