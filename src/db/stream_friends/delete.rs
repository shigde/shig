use crate::db::error::DbResult;
use diesel::prelude::*;

#[allow(dead_code)]
pub fn delete_stream_friend_by_id(conn: &mut PgConnection, friend: i32) -> DbResult<()> {
    use crate::db::schema::stream_friends::dsl::stream_friends;
    use crate::db::schema::stream_friends::id;

    diesel::delete(stream_friends.filter(id.eq(friend))).execute(conn)?;
    Ok(())
}
