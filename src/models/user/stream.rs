use crate::db::channels::read::find_channel_by_id;
use crate::db::streams::delete::delete_stream_by_id;
use crate::db::streams::read::find_stream_by_uuid;
use crate::db::users::read::find_user_by_id;
use crate::db::DbPool;
use crate::models::auth::session::Principal;
use crate::models::error::ApiError;
use crate::models::user::stream_meta_data::StreamMetaData;
use actix_web::web::Data;
use chrono::NaiveDateTime;
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

impl Stream {
    pub(crate) fn fetch(
        pool: &Data<DbPool>,
        stream_uuid: String,
        principal: Principal,
    ) -> Result<Stream, ApiError> {
        let mut conn = pool.get()?;
        let stream_dao = find_stream_by_uuid(&mut conn, stream_uuid.clone())?;

        // Check if user is the owner of the stream
        if principal.id != stream_dao.stream.user_id {
            return Err(ApiError::Unauthorized {
                error_message: "unauthorized".to_string(),
            });
        }

        let channel = find_channel_by_id(&mut conn, stream_dao.stream.channel_id)?;
        let user = find_user_by_id(&mut conn, stream_dao.stream.user_id)?;
        Ok(Self::from_dao(
            stream_dao,
            user.user_uuid.as_str(),
            channel.uuid.as_str(),
        ))
    }

    pub(crate) fn delete(
        pool: &Data<DbPool>,
        stream_uuid: String,
        principal: Principal,
    ) -> Result<(), ApiError> {
        let mut conn = pool.get()?;
        let stream_dao = find_stream_by_uuid(&mut conn, stream_uuid.clone())?;

        // Check if user is the owner of the stream
        if principal.id != stream_dao.stream.user_id {
            return Err(ApiError::Unauthorized {
                error_message: "unauthorized".to_string(),
            });
        }
        delete_stream_by_id(&mut conn, stream_dao.stream.id)?;
        Ok(())
    }

    pub(crate) fn from_dao(
        dao: crate::db::streams::read::FullyLoadedStream,
        owner_uuid: &str,
        channel_uuid: &str,
    ) -> Stream {
        let thumbnail_url = match dao.thumbnail {
            None => "".to_string(),
            Some(img) => img.file_url.unwrap_or("".to_string()),
        };
        let meta_data = StreamMetaData::from_dao(dao.meta_data);
        Stream {
            uuid: dao.stream.uuid,
            title: dao.stream.title,
            thumbnail: thumbnail_url,
            description: dao.stream.description.unwrap_or("".to_string()),
            support: dao.stream.support.unwrap_or("".to_string()),
            date: dao.stream.date,
            start_time: dao.stream.start_time,
            end_time: dao.stream.end_time,
            viewer: 0,
            likes: 0,
            dislikes: 0,
            licence: dao.stream.licence,
            is_repeating: dao.stream.is_repeating,
            repeat: dao.stream.repeat.unwrap_or(0),
            meta_data,
            is_live: dao.stream.is_live,
            is_public: dao.stream.is_public,
            owner_uuid: owner_uuid.to_string(),
            channel_uuid: channel_uuid.to_string(),
        }
    }
}
