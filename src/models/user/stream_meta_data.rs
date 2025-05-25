use crate::db::stream_meta_data::create::NewStreamMetaData;
use crate::db::stream_meta_data::update::StreamMetaDataUpdate;
use crate::db::stream_meta_data::StreamMetaData as StreamMetaDataDAO;

use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StreamMetaData {
    pub is_shig: bool,
    pub stream_key: String,
    pub url: String,
    pub protocol: StreamProtocol,
    pub permanent_live: bool,
    pub save_replay: bool,
    pub latency_mode: StreamLatency,
}

impl StreamMetaData {
    pub(crate) fn from_dao(dao: StreamMetaDataDAO) -> StreamMetaData {
        StreamMetaData {
            is_shig: dao.is_shig,
            stream_key: dao.stream_key,
            url: dao.url,
            protocol: StreamProtocol::from_integer(dao.protocol),
            permanent_live: dao.permanent_live,
            save_replay: dao.save_replay,
            latency_mode: StreamLatency::from_integer(dao.latency_mode),       
        }
    }
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
    pub fn from_integer(value: i32) -> StreamProtocol {
        match value {
            1 => StreamProtocol::RTMP,
            2 => StreamProtocol::WHIP,
            3 => StreamProtocol::MOQ,
            _ => StreamProtocol::RTMP,
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
    pub fn from_integer(value: i32) -> StreamLatency {
        match value {
            1 => StreamLatency::LOW,
            2 => StreamLatency::STANDARD,
            3 => StreamLatency::HIGH,
            _ => StreamLatency::STANDARD,
        }
    }   
}
