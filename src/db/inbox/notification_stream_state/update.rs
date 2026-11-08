use crate::db::{error::DbResult, schema::notification_stream_state};
use diesel::prelude::*;

pub fn set_live(conn: &mut PgConnection, stream_id: i32, live: bool) -> DbResult<()> {
    diesel::update(notification_stream_state::table.find(stream_id))
        .set(notification_stream_state::live.eq(live))
        .execute(conn)?;
    Ok(())
}
