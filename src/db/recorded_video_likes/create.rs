use crate::db::error::DbResult;
use chrono::{NaiveDateTime, Utc};
use diesel::{prelude::*, PgConnection};

#[derive(Insertable)]
#[diesel(table_name = crate::db::schema::recorded_video_likes)]
struct NewRecordedVideoLike {
    recorded_video_id: i32,
    user_id: i32,
    created_at: NaiveDateTime,
}

pub fn insert(conn: &mut PgConnection, recorded_video_id: i32, user_id: i32) -> DbResult<()> {
    use crate::db::schema::recorded_video_likes::dsl as likes;

    diesel::insert_into(likes::recorded_video_likes)
        .values(NewRecordedVideoLike {
            recorded_video_id,
            user_id,
            created_at: Utc::now().naive_utc(),
        })
        .on_conflict((likes::recorded_video_id, likes::user_id))
        .do_nothing()
        .execute(conn)?;
    Ok(())
}
