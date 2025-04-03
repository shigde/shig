use crate::db::channels::Channel;
use crate::db::error::DbResult;
use diesel::{QueryDsl, RunQueryDsl, SelectableHelper, PgConnection};
use diesel::prelude::*;

pub fn find_channel_by_user_id(conn: &mut PgConnection, find_user_id: i32) -> DbResult<Channel> {
    use crate::db::schema::channels::dsl::channels;
    use crate::db::schema::channels::user_id;
    let chan = channels
        .filter(user_id.eq(find_user_id))
        .select(Channel::as_select())
        .first(conn)?;

    Ok(chan)
}

#[allow(dead_code)]
pub fn find_channel_by_id(conn: &mut PgConnection, find_channel_id: i32) -> DbResult<Channel> {
    use crate::db::schema::channels::dsl::channels;
    use crate::db::schema::channels::id;
    let chan = channels
        .filter(id.eq(find_channel_id))
        .select(Channel::as_select())
        .first(conn)?;

    Ok(chan)
}

#[allow(dead_code)]
pub fn find_channel_by_actor(conn: &mut PgConnection, find_actor_id: i32) -> DbResult<Channel> {
    use crate::db::schema::channels::dsl::channels;
    use crate::db::schema::channels::actor_id;
    let chan = channels
        .filter(actor_id.eq(find_actor_id))
        .select(Channel::as_select())
        .first(conn)?;

    Ok(chan)
}
