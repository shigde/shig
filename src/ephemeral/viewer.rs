use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const VIEWER_TTL: Duration = Duration::from_secs(60);
const MAX_VIEWERS_PER_STREAM: usize = 5000;

/// Ephemeral viewer presence, tracked independently for each stream.
#[derive(Clone, Default)]
pub struct Viewer(Arc<Mutex<HashMap<String, HashMap<String, Instant>>>>);

impl Viewer {
    pub fn clear_stream(&self, stream_uuid: &str) {
        self.0
            .lock()
            .expect("viewer presence lock poisoned")
            .remove(stream_uuid);
    }

    pub fn heartbeat(&self, stream_uuid: &str, viewer_key: &str) -> usize {
        let now = Instant::now();
        let mut streams = self.0.lock().expect("viewer presence lock poisoned");
        streams.retain(|_, viewers| {
            viewers.retain(|_, last_seen| now.duration_since(*last_seen) < VIEWER_TTL);
            !viewers.is_empty()
        });

        let viewers = streams.entry(stream_uuid.to_owned()).or_default();
        if viewers.contains_key(viewer_key) || viewers.len() < MAX_VIEWERS_PER_STREAM {
            viewers.insert(viewer_key.to_owned(), now);
        }
        viewers.len()
    }
}
