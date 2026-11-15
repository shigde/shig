use crate::db::error::DbResult;
use diesel::{sql_types::{Integer, Text}, PgConnection, RunQueryDsl};

pub fn merge(conn: &mut PgConnection, user_id: i32, settings: &str) -> DbResult<usize> {
    Ok(diesel::sql_query("UPDATE user_stream_settings SET settings=settings || $2::jsonb, updated_at=now() WHERE user_id=$1")
        .bind::<Integer, _>(user_id)
        .bind::<Text, _>(settings)
        .execute(conn)?)
}
