use crate::db::error::DbResult;
use diesel::prelude::*;

#[allow(dead_code)]
pub fn delete_stream_friend_by_id(conn: &mut PgConnection, friend: i32) -> DbResult<()> {
    use crate::db::schema::channel_friends::dsl::channel_friends;
    use crate::db::schema::channel_friends::id;

    diesel::delete(channel_friends.filter(id.eq(friend))).execute(conn)?;
    Ok(())
}
