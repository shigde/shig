use super::Notification;
use crate::db::error::DbResult;
use diesel::{
    prelude::*,
    sql_types::{Integer, Nullable, Text},
};

pub fn insert_for_active_recipient(
    conn: &mut PgConnection,
    notification: &Notification<'_>,
) -> DbResult<()> {
    diesel::sql_query(
        "INSERT INTO notifications (id, recipient_id, stream_id, action_path, event_key, title, body)
         SELECT $1, $2, $3, $4, $5, $6, $7
         WHERE EXISTS (SELECT 1 FROM users WHERE id = $2 AND active)
             AND ($3::integer IS NULL OR $2 <> (SELECT user_id FROM streams WHERE id = $3))
         ON CONFLICT (recipient_id, event_key) DO NOTHING",
    )
    .bind::<Text, _>(&notification.id)
    .bind::<Integer, _>(notification.recipient_id)
    .bind::<Nullable<Integer>, _>(notification.stream_id)
    .bind::<Nullable<Text>, _>(notification.action_path)
    .bind::<Text, _>(notification.event_key)
    .bind::<Text, _>(notification.title)
    .bind::<Text, _>(notification.body)
    .execute(conn)?;
    Ok(())
}

pub fn insert_for_active_admins(
    conn: &mut PgConnection,
    id: &str,
    exclude_user_id: i32,
    event_key: &str,
    title: &str,
    body: &str,
    action_path: &str,
) -> DbResult<()> {
    diesel::sql_query(
        "INSERT INTO notifications (id, recipient_id, stream_id, action_path, event_key, title, body)
         SELECT md5($1 || ':' || admin.id::text), admin.id, NULL, $5, $2, $3, $4
         FROM users admin
         WHERE admin.active AND admin.user_role_id = 1 AND admin.id <> $6
         ON CONFLICT (recipient_id, event_key) DO NOTHING",
    )
    .bind::<Text, _>(id)
    .bind::<Text, _>(event_key)
    .bind::<Text, _>(title)
    .bind::<Text, _>(body)
    .bind::<Text, _>(action_path)
    .bind::<Integer, _>(exclude_user_id)
    .execute(conn)?;
    Ok(())
}
