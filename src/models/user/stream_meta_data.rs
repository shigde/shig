use crate::db::stream_meta_data::create::NewStreamMetaData;
use crate::db::stream_meta_data::update::StreamMetaDataUpdate;

use chrono::Utc;
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

impl StreamMetaData {
    pub fn build_insert_dao(&self, stream_id: i32) -> NewStreamMetaData {
        NewStreamMetaData {
            stream_id,
            is_shig: self.is_shig,
            stream_key: &self.stream_key,
            url: &self.url,
            protocol: self.protocol.value_as_integer(),
            permanent_live: self.permanent_live,
            save_replay: self.save_replay,
            latency_mode: self.latency_mode.value_as_integer(),
            created_at: Utc::now().naive_utc(),
        }
    }

    pub fn build_update_dao(&self, stream_id: i32) -> StreamMetaDataUpdate {
        StreamMetaDataUpdate {
            stream_id,
            is_shig: self.is_shig,
            stream_key: &self.stream_key,
            url: &self.url,
            protocol: self.protocol.value_as_integer(),
            permanent_live: self.permanent_live,
            save_replay: self.save_replay,
            latency_mode: self.latency_mode.value_as_integer(),
        }
    }
}

impl StreamProtocol {
    pub fn value_as_integer(&self) -> i32 {
        match &self {
            StreamProtocol::RTMP => 1,
            StreamProtocol::WHIP => 2,
            StreamProtocol::MOQ => 3,
        }
    }
}

impl StreamLatency {
    pub fn value_as_integer(&self) -> i32 {
        match &self {
            StreamLatency::LOW => 1,
            StreamLatency::STANDARD => 2,
            StreamLatency::HIGH => 3,
        }
    }
}
