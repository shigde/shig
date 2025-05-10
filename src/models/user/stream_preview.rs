use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StreamPreview {
    pub uuid: String,
    pub title: String,
    pub thumbnail: String,
    pub description: String,
    pub support: String,
    pub date: NaiveDateTime,
    pub start_time: Option<NaiveDateTime>,
    pub end_time: Option<NaiveDateTime>,
    pub viewer: u64,
    pub likes: u64,
    pub dislikes: u64,
    pub is_live: bool,
    pub is_public: bool,
    pub owner_name: String,
    pub owner_uuid: String,
    pub owner_avatar: String,
    pub channel_name: String,
    pub channel_uuid: String,
}
