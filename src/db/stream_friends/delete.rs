use crate::db::error::DbResult;
use diesel::prelude::*;

#[allow(dead_code)]
pub fn delete_stream_friend_by_id(conn: &mut PgConnection, friend: i32) -> DbResult<()> {
    use crate::db::schema::stream_friends::dsl::stream_friends;
    use crate::db::schema::stream_friends::id;

    diesel::delete(stream_friends.filter(id.eq(friend))).execute(conn)?;
    Ok(())
}

pub fn delete_stream_friend_by_user_and_stream_uuid(
    conn: &mut PgConnection,
    stream_uuid: &str,
    user_uuid: &str,
) -> DbResult<usize> {
    use crate::db::schema::{
        stream_friends::dsl as sf,
        users::dsl as u,
        streams::dsl as s,
    };

    // Subquery: find user_id
    let user_id_subquery = u::users
        .filter(u::user_uuid.eq(user_uuid))
        .select(u::id);

    // Subquery: find stream_id
    let stream_id_subquery = s::streams
        .filter(s::uuid.eq(stream_uuid))
        .select(s::id);

    let result = diesel::delete(
        sf::stream_friends
            .filter(sf::user_id.eq_any(user_id_subquery))
            .filter(sf::stream_id.eq_any(stream_id_subquery)),
    ).execute(conn)?;

    Ok(result)
}