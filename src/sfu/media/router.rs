use crate::sfu::media::{Media, MediaId};
use crate::sfu::peer::PeerId;
use std::collections::HashMap;

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
}
