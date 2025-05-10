use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StreamMetaData {
    pub is_shig: bool,
    pub stream_key: String,
    url: String,
    pub protocol: StreamProtocol,
    permanent_live: bool,
    save_replay: bool,
    latency_mode: StreamLatency,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum StreamProtocol {
    RTMP = 1,
    WHIP = 2,
    MOQ = 3,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum StreamLatency {
    LOW = 1,
    STANDARD = 2,
    HIGH = 3,
}
