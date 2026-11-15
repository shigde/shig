use crate::db::error::DbResult;
use diesel::{prelude::*, PgConnection};

pub fn remove(conn: &mut PgConnection, recorded_video_id: i32, user_id: i32) -> DbResult<()> {
    use crate::db::schema::recorded_video_likes::dsl as likes;

    diesel::delete(
        likes::recorded_video_likes
            .filter(likes::recorded_video_id.eq(recorded_video_id))
            .filter(likes::user_id.eq(user_id)),
    )
    .execute(conn)?;
    Ok(())
}
