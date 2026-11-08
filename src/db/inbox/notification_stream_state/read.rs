use super::NotificationStreamState;
use crate::db::{error::DbResult, schema::notification_stream_state};
use diesel::prelude::*;

pub fn lock(conn: &mut PgConnection, stream_id: i32) -> DbResult<NotificationStreamState> {
    Ok(notification_stream_state::table
        .find(stream_id)
        .select(NotificationStreamState::as_select())
        .for_update()
        .first(conn)?)
}
