use super::Message;
use crate::db::{error::DbResult, schema::messages};
use diesel::prelude::*;

pub fn insert(conn: &mut PgConnection, message: &Message<'_>) -> DbResult<()> {
    diesel::insert_into(messages::table)
        .values(message)
        .on_conflict((messages::recipient_id, messages::event_key))
        .do_nothing()
        .execute(conn)?;
    Ok(())
}
