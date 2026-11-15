use crate::db::error::DbResult;
use diesel::{sql_types::{Integer, Text}, PgConnection, RunQueryDsl};

pub fn insert(conn: &mut PgConnection, user_id: i32, settings: &str) -> DbResult<()> {
    diesel::sql_query("INSERT INTO user_stream_settings(user_id, settings) VALUES($1, $2::jsonb) ON CONFLICT(user_id) DO UPDATE SET settings=user_stream_settings.settings || EXCLUDED.settings, updated_at=now()")
        .bind::<Integer, _>(user_id)
        .bind::<Text, _>(settings)
        .execute(conn)?;
    Ok(())
}
