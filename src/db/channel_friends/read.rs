use crate::db::channel_friends::ChannelFriend;
use crate::db::error::DbResult;
use diesel::dsl::exists;
use diesel::prelude::*;
use diesel::{select, PgConnection, QueryDsl, RunQueryDsl, SelectableHelper};

#[allow(dead_code)]
pub fn find_by_channel_user(
    conn: &mut PgConnection,
    channel: i32,
    user: i32,
) -> DbResult<ChannelFriend> {
    use crate::db::schema::channel_friends::channel_id;
    use crate::db::schema::channel_friends::dsl::channel_friends;
    use crate::db::schema::channel_friends::user_id;
    let friend = channel_friends
        .filter(channel_id.eq(channel.to_owned()))
        .filter(user_id.eq(user.to_owned()))
        .select(ChannelFriend::as_select())
        .first(conn)?;

    Ok(friend)
}

#[allow(dead_code)]
pub fn find_all_by_channel(conn: &mut PgConnection, channel: i32) -> DbResult<Vec<ChannelFriend>> {
    use crate::db::schema::channel_friends::channel_id;
    use crate::db::schema::channel_friends::dsl::channel_friends;
    let friends = channel_friends
        .filter(channel_id.eq(channel.to_owned()))
        .select(ChannelFriend::as_select())
        .load(conn)?;
    Ok(friends)
}

#[allow(dead_code)]
pub fn is_channel_friend(conn: &mut PgConnection, channel: i32, user: i32) -> DbResult<bool> {
    use crate::db::schema::channel_friends::dsl::*;
    let exists = select(exists(
        channel_friends
            .filter(user_id.eq(user))
            .filter(channel_id.eq(channel))
            .filter(active.eq(true))
            .filter(accepted.eq(true)),
    ))
    .get_result(conn)?;
    Ok(exists)
}
