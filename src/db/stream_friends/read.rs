use crate::db::error::DbResult;
use crate::db::stream_friends::StreamFriend;
use diesel::dsl::exists;
use diesel::prelude::*;
use diesel::{select, PgConnection, QueryDsl, RunQueryDsl, SelectableHelper};
use crate::db::active_users::ActiveUser;

pub fn find_active_stream_friend_by_uuids(
    conn: &mut PgConnection,
    stream_uuid: &str,
    user_uuid: &str,
) -> DbResult<Option<ActiveUser>> {
    use crate::db::schema::{
        stream_friends::dsl as sf,
        streams::dsl as s,
    };

    use crate::db::schema_views::active_users::dsl as au;

    let friend = sf::stream_friends
        .inner_join(s::streams.on(sf::stream_id.eq(s::id)))
        .inner_join(au::active_users.on(sf::user_id.eq(au::id)))
        .filter(s::uuid.eq(stream_uuid))
        .filter(au::user_uuid.eq(user_uuid))
        .filter(sf::active.eq(true))
        .filter(sf::accepted.eq(true))
        .select(au::active_users::all_columns())
        .first::<ActiveUser>(conn)
        .optional()?;
    
    Ok(friend)
}

pub fn find_all_active_stream_friends_by_stream_uuid(
    conn: &mut PgConnection,
    stream_uuid: &str,
) -> DbResult<Vec<ActiveUser>> {
    use crate::db::schema::{
        stream_friends::dsl as sf,
        streams::dsl as s,
    };

    use crate::db::schema_views::active_users::dsl as au;

    let active_user = sf::stream_friends
        .inner_join(s::streams.on(sf::stream_id.eq(s::id)))
        .inner_join(au::active_users.on(sf::user_id.eq(au::id)))
        .filter(s::uuid.eq(stream_uuid))
        .select(au::active_users::all_columns())
        .load::<ActiveUser>(conn)?;

    Ok(active_user)
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
