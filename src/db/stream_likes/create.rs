use crate::db::error::DbResult;
use chrono::{NaiveDateTime, Utc};
use diesel::{prelude::*, PgConnection};

#[derive(Insertable)]
#[diesel(table_name = crate::db::schema::stream_likes)]
struct NewStreamLike {
    stream_id: i32,
    user_id: i32,
    created_at: NaiveDateTime,
}

pub fn insert(conn: &mut PgConnection, stream_id: i32, user_id: i32) -> DbResult<()> {
    use crate::db::schema::stream_likes::dsl as likes;

    diesel::insert_into(likes::stream_likes)
        .values(NewStreamLike {
            stream_id,
            user_id,
            created_at: Utc::now().naive_utc(),
        })
        .on_conflict((likes::stream_id, likes::user_id))
        .do_nothing()
        .execute(conn)?;
    Ok(())
}
