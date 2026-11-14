use crate::db::error::DbResult;
use diesel::{prelude::*, PgConnection};

pub fn remove(conn: &mut PgConnection, stream_id: i32, user_id: i32) -> DbResult<()> {
    use crate::db::schema::stream_likes::dsl as likes;

    diesel::delete(
        likes::stream_likes
            .filter(likes::stream_id.eq(stream_id))
            .filter(likes::user_id.eq(user_id)),
    )
    .execute(conn)?;
    Ok(())
}
