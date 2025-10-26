use crate::db::error::DbResult;
use crate::db::stream_friends::StreamFriend;
use diesel::dsl::exists;
use diesel::prelude::*;
use diesel::{select, PgConnection, QueryDsl, RunQueryDsl, SelectableHelper};

#[allow(dead_code)]
pub fn find_by_stream_user(
    conn: &mut PgConnection,
    stream: i32,
    user: i32,
) -> DbResult<StreamFriend> {
    use crate::db::schema::stream_friends::dsl::stream_friends;
    use crate::db::schema::stream_friends::stream_id;
    use crate::db::schema::stream_friends::user_id;
    let friend = stream_friends
        .filter(stream_id.eq(stream.to_owned()))
        .filter(user_id.eq(user.to_owned()))
        .select(StreamFriend::as_select())
        .first(conn)?;

    Ok(friend)
}

#[allow(dead_code)]
pub fn find_all_by_stream(conn: &mut PgConnection, stream: i32) -> DbResult<Vec<StreamFriend>> {
    use crate::db::schema::stream_friends::dsl::stream_friends;
    use crate::db::schema::stream_friends::stream_id;
    let friends = stream_friends
        .filter(stream_id.eq(stream.to_owned()))
        .select(StreamFriend::as_select())
        .load(conn)?;
    Ok(friends)
}

pub fn is_stream_friend(conn: &mut PgConnection, stream: i32, user: i32) -> DbResult<bool> {
    use crate::db::schema::stream_friends::dsl::*;
    let exists = select(exists(
        stream_friends
            .filter(user_id.eq(user))
            .filter(stream_id.eq(stream))
            .filter(active.eq(true))
            .filter(accepted.eq(true)),
    ))
    .get_result(conn)?;
    Ok(exists)
}
