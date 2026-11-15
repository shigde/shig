use crate::{
    db::{
        recorded_video_likes,
        recorded_videos,
        streams,
        stream_likes,
    },
    models::error::ApiError,
};
use diesel::{Connection, PgConnection};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReactionStats {
    pub likes: i64,
    pub liked_by_me: bool,
}

#[derive(Clone, Copy)]
enum ContentId {
    Stream(i32),
    Video(i32),
}

fn content_id(conn: &mut PgConnection, kind: &str, uuid: &str) -> Result<ContentId, ApiError> {
    match kind {
        "stream" => {
            let stream = streams::read::find_stream_by_uuid(conn, uuid.to_owned())?;
            Ok(ContentId::Stream(stream.id))
        }
        "video" => {
            let video_id = recorded_videos::read::find_id_by_uuid(conn, uuid)?;
            Ok(ContentId::Video(video_id))
        }
        _ => Err(ApiError::BadRequest {
            error_message: "Invalid content type".into(),
        }),
    }
}

pub fn set(
    conn: &mut PgConnection,
    kind: &str,
    uuid: &str,
    user_id: i32,
    liked: bool,
) -> Result<ReactionStats, ApiError> {
    conn.transaction::<_, ApiError, _>(|conn| {
        let id = content_id(conn, kind, uuid)?;
        match (id, liked) {
            (ContentId::Stream(stream_id), true) => {
                stream_likes::create::insert(conn, stream_id, user_id)?;
            }
            (ContentId::Stream(stream_id), false) => {
                stream_likes::delete::remove(conn, stream_id, user_id)?;
            }
            (ContentId::Video(video_id), true) => {
                recorded_video_likes::create::insert(conn, video_id, user_id)?;
            }
            (ContentId::Video(video_id), false) => {
                recorded_video_likes::delete::remove(conn, video_id, user_id)?;
            }
        }
        stats_for_content(conn, id, user_id)
    })
}

#[allow(dead_code)]
pub fn is_like(
    conn: &mut PgConnection,
    kind: &str,
    uuid: &str,
    user_id: i32,
) -> Result<bool, ApiError> {
    let liked = match content_id(conn, kind, uuid)? {
        ContentId::Stream(stream_id) => {
            stream_likes::read::exists_for_user(conn, stream_id, user_id)?
        }
        ContentId::Video(video_id) => {
            recorded_video_likes::read::exists_for_user(conn, video_id, user_id)?
        }
    };
    Ok(liked)
}

pub fn stats(
    conn: &mut PgConnection,
    kind: &str,
    uuid: &str,
    user_id: i32,
) -> Result<ReactionStats, ApiError> {
    let id = content_id(conn, kind, uuid)?;
    stats_for_content(conn, id, user_id)
}

fn stats_for_content(
    conn: &mut PgConnection,
    content: ContentId,
    user_id: i32,
) -> Result<ReactionStats, ApiError> {
    let (likes, liked_by_me) = match content {
        ContentId::Stream(stream_id) => stream_likes::read::stats_for_user(conn, stream_id, user_id)?,
        ContentId::Video(video_id) => {
            recorded_video_likes::read::stats_for_user(conn, video_id, user_id)?
        }
    };
    Ok(ReactionStats { likes, liked_by_me })
}

