use crate::relay::publisher::VideoStructure;
use std::sync::atomic::{AtomicU32, AtomicU64};
use std::sync::Arc;

/// Shared publisher stats readable from the stats loop via Arc.
pub struct PublisherStats {
    pub bytes_published: AtomicU64,
    pub frames_sent: AtomicU64,
    pub segments_sent: AtomicU64,
    pub track_count: AtomicU32,
    pub video_width: AtomicU32,
    pub video_height: AtomicU32,
    pub video_codec: std::sync::Mutex<String>,
    pub audio_codec: std::sync::Mutex<String>,
    /// Latest catalog JSON with initData stripped out.
    pub catalog_json: std::sync::Mutex<Option<serde_json::Value>>,
    /// Transport-level stats (QUIC/WebTransport), updated periodically.
    pub transport: std::sync::Mutex<Option<serde_json::Value>>,
    /// Video structure from the latest completed segment.
    pub video_structure: std::sync::Mutex<Option<VideoStructure>>,
}

impl PublisherStats {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            bytes_published: AtomicU64::new(0),
            frames_sent: AtomicU64::new(0),
            segments_sent: AtomicU64::new(0),
            track_count: AtomicU32::new(0),
            video_width: AtomicU32::new(0),
            video_height: AtomicU32::new(0),
            video_codec: std::sync::Mutex::new(String::new()),
            audio_codec: std::sync::Mutex::new(String::new()),
            catalog_json: std::sync::Mutex::new(None),
            transport: std::sync::Mutex::new(None),
            video_structure: std::sync::Mutex::new(None),
        })
    }
}
