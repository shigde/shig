use super::Notification;
use crate::db::error::DbResult;
use diesel::{
    prelude::*,
    sql_types::{Integer, Text},
};

pub fn insert_for_active_recipient(
    conn: &mut PgConnection,
    notification: &Notification<'_>,
) -> DbResult<()> {
    diesel::sql_query(
        "INSERT INTO notifications (id, recipient_id, stream_id, event_key, title, body)
         SELECT $1, $2, $3, $4, $5, $6
         WHERE EXISTS (SELECT 1 FROM users WHERE id = $2 AND active)
             AND $2 <> (SELECT user_id FROM streams WHERE id = $3)
         ON CONFLICT (recipient_id, event_key) DO NOTHING",
    )
    .bind::<Text, _>(&notification.id)
    .bind::<Integer, _>(notification.recipient_id)
    .bind::<Integer, _>(notification.stream_id)
    .bind::<Text, _>(notification.event_key)
    .bind::<Text, _>(notification.title)
    .bind::<Text, _>(notification.body)
    .execute(conn)?;
    Ok(())
}
