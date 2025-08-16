use crate::sfu::peer::PeerId;
use derive_more::Display;
use std::sync::Arc;
use webrtc::track::track_local::TrackLocal;

pub mod connector;
pub mod data_channel;
pub mod error;
pub mod message;
pub mod receiver;
pub mod router;
pub mod sender;

#[derive(Clone)]
pub struct Media {
    pub id: MediaId,
    pub peer_id: PeerId,
    pub track: Arc<dyn TrackLocal + Send + Sync>,
    pub kind: MediaKind,
}

impl Media {
    pub fn new(peer_id: PeerId, track: Arc<dyn TrackLocal + Send + Sync>, kind: MediaKind) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        Self {
            id: MediaId::from(id),
            peer_id,
            track,
            kind,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Display)]
pub struct MediaId(String);

impl From<String> for MediaId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for MediaId {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

#[derive(Clone, Copy)]
pub enum MediaKind {
    Audio,
    Video,
}
