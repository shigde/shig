use crate::db::{error::DbResult, schema::notification_stream_state};
use diesel::prelude::*;

pub fn ensure(conn: &mut PgConnection, stream_id: i32) -> DbResult<()> {
    diesel::insert_into(notification_stream_state::table)
        .values(notification_stream_state::stream_id.eq(stream_id))
        .on_conflict(notification_stream_state::stream_id)
        .do_nothing()
        .execute(conn)?;
    Ok(())
}
