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
    /// Returns all media *not* belonging to this peer.
    pub fn get_medias_without_peer(&self, peer_id: &PeerId) -> Vec<Media> {
        self.medias
            .values()
            .filter(|&media| &media.peer_id != peer_id)
            .cloned()
            .collect()
    }

    #[allow(dead_code)]
    /// Returns all media for a specific peer.
    pub fn get_medias_of_peer(&self, peer_id: &PeerId) -> Vec<Media> {
        self.medias
            .values()
            .filter(|&media| &media.peer_id == peer_id)
            .cloned()
            .collect()
    }

    #[allow(dead_code)]
    /// Removes all media of a specific peer.
    pub fn remove_medias_of_peer(&mut self, peer_id: &PeerId) {
        let keys_to_remove: Vec<MediaId> = self
            .medias
            .iter()
            .filter(|(_, media)| &media.peer_id == peer_id)
            .map(|(id, _)| id.clone()) // statt *id → clone()
            .collect();

        for key in keys_to_remove {
            self.medias.remove(&key);
        }
    }
}
