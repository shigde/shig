use crate::db::error::DbResult;
use diesel::prelude::*;

pub fn delete_channel_by_id(conn: &mut PgConnection, channel_id: i32) -> DbResult<()> {
    use crate::db::schema::channels::dsl::channels;
    use crate::db::schema::channels::id;

    diesel::delete(channels.filter(id.eq(channel_id))).execute(conn)?;
    Ok(())
}
