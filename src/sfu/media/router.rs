use crate::sfu::media::{Media, MediaId};
use crate::sfu::peer::PeerId;
use std::collections::HashMap;
#[allow(dead_code)]
pub struct Router {
    pub medias: HashMap<MediaId, Media>,
}
#[allow(dead_code)]
impl Router {
    pub fn new() -> Self {
        Self {
            medias: HashMap::new(),
        }
    }
    #[allow(dead_code)]
    pub fn get_medias_without_peer(&mut self, peer_id: PeerId) -> Vec<Media> {
        self.medias
            .values()
            .filter(|&val| val.peer_id != peer_id)
            .map(|media| media)
            .cloned()
            .collect()
    }
    #[allow(dead_code)]
    pub fn get_medias_of_peer(&mut self, peer_id: PeerId) -> Vec<Media> {
        self.medias
            .values()
            .filter(|&val| val.peer_id == peer_id)
            .map(|media| media)
            .cloned()
            .collect()
    }
    #[allow(dead_code)]
    pub fn remove_medias_of_peer(&mut self, peer_id: PeerId) {
        let keys: Vec<MediaId> = self
            .medias
            .iter() // &(&MediaId, &Media)
            .filter(|(_, media)| media.peer_id == peer_id)
            .map(|(id, _)| id)
            .cloned() // id ist &MediaId, dereferenzieren
            .collect();

        for key in keys {
            self.medias.remove(&key);
        }
    }
}
