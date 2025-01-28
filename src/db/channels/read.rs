use crate::db::channels::Channel;
use crate::db::error::DbResult;
use diesel::{QueryDsl, RunQueryDsl, SelectableHelper, SqliteConnection};
use diesel::prelude::*;

pub fn find_channel_by_user_id(conn: &mut SqliteConnection, find_user_id: i32) -> DbResult<Channel> {
    use crate::db::schema::channels::dsl::channels;
    use crate::db::schema::channels::user_id;
    let chan = channels
        .filter(user_id.eq(find_user_id))
        .select(Channel::as_select())
        .first(conn)?;

    Ok(chan)
}
