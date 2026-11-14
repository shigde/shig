use crate::db::error::DbResult;
use diesel::{prelude::*, PgConnection};

pub fn find_id_by_uuid(conn: &mut PgConnection, video_uuid: &str) -> DbResult<i32> {
    use crate::db::schema::recorded_videos::dsl as videos;

    Ok(videos::recorded_videos
        .filter(videos::uuid.eq(video_uuid))
        .select(videos::id)
        .first(conn)?)
}
