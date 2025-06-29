use crate::db::stream_previews::read::{find_all_stream_previews, find_all_stream_previews_by_channel, find_stream_preview_by_uuid};
use crate::db::stream_previews::StreamPreview as StreamPreviewDAO;
use crate::db::DbPool;
use crate::models::error::ApiError;
use actix_web::web::Data;
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

impl StreamPreview {
    pub(crate) fn fetch(
        pool: &Data<DbPool>,
        stream_uuid: String,
    ) -> Result<StreamPreview, ApiError> {
        let mut conn = pool.get()?;
        let stream = find_stream_preview_by_uuid(&mut conn, stream_uuid)?;
        Ok(Self::from_dao(stream))
    }

    pub(crate) fn fetch_all(pool: &Data<DbPool>) -> Result<Vec<StreamPreview>, ApiError> {
        let mut conn = pool.get()?;
        let streams = find_all_stream_previews(&mut conn)?;
        let return_list = streams.into_iter().map(|x| Self::from_dao(x)).collect();
        Ok(return_list)
    }

    pub(crate) fn fetch_all_by_channel(pool: &Data<DbPool>, channel_uuid: String,) -> Result<Vec<StreamPreview>, ApiError> {
        let mut conn = pool.get()?;
        let streams = find_all_stream_previews_by_channel(&mut conn, channel_uuid)?;
        let return_list = streams.into_iter().map(|x| Self::from_dao(x)).collect();
        Ok(return_list)
    }

    fn from_dao(dao: StreamPreviewDAO) -> Self {
        StreamPreview {
            uuid: dao.uuid.to_string(),
            title: dao.title.to_string(),
            thumbnail: dao.thumbnail.unwrap_or("".to_string()),
            description: dao.description.unwrap_or("".to_string()),
            support: dao.support.unwrap_or("".to_string()),
            date: dao.date,
            start_time: dao.start_time,
            end_time: dao.end_time,
            viewer: 0,
            likes: 0,
            dislikes: 0,
            is_live: dao.is_live,
            is_public: dao.is_public,
            owner_name: dao.owner_name,
            owner_uuid: dao.owner_uuid,
            owner_avatar: dao.owner_avatar.unwrap_or("".to_string()),
            channel_name: dao.channel_name,
            channel_uuid: dao.channel_uuid,
        }
    }
}
