use crate::models::user::stream_meta_data::StreamMetaData;
use chrono::{NaiveDateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Stream {
    pub uuid: String,
    pub title: String,
    pub thumbnail: String,
    pub description: String,
    pub support: String,
    pub date: NaiveDateTime,
    pub start_time: Option<NaiveDateTime>,
    pub end_time: Option<NaiveDateTime>,
    pub viewer: i64,
    pub likes: i64,
    pub dislikes: i64,
    pub licence: i32,
    pub is_repeating: bool,
    pub repeat: i32,
    pub meta_data: StreamMetaData,
    pub is_live: bool,
    pub is_public: bool,
    pub owner_uuid: String,
    pub channel_uuid: String,
}
