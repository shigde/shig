pub mod create;
pub mod read;
pub mod update;

use crate::db::schema::notification_stream_state;
use diesel::prelude::*;

#[derive(Queryable, Identifiable, Selectable)]
#[diesel(table_name = notification_stream_state)]
#[diesel(primary_key(stream_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NotificationStreamState {
    pub stream_id: i32,
    pub live: bool,
}
