use super::Inbox;
use crate::db::error::DbResult;
use diesel::{
    prelude::*,
    sql_types::{Integer, Text},
};

pub fn mark_read(conn: &mut PgConnection, inbox: Inbox, user: i32, id: &str) -> DbResult<bool> {
    let updated = diesel::sql_query(format!(
        "UPDATE {} SET read_at = COALESCE(read_at, now()) WHERE recipient_id = $1 AND id = $2",
        inbox.table()
    ))
    .bind::<Integer, _>(user)
    .bind::<Text, _>(id)
    .execute(conn)?;
    Ok(updated > 0)
}

pub fn mark_all_read(conn: &mut PgConnection, inbox: Inbox, user: i32) -> DbResult<()> {
    diesel::sql_query(format!(
        "UPDATE {} SET read_at = now() WHERE recipient_id = $1 AND read_at IS NULL",
        inbox.table()
    ))
    .bind::<Integer, _>(user)
    .execute(conn)?;
    Ok(())
}
