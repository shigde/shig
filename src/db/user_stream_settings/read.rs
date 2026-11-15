use crate::db::error::DbResult;
use diesel::{sql_types::{Integer, Text}, PgConnection, OptionalExtension, RunQueryDsl};

#[derive(diesel::QueryableByName)]
struct SettingsRow {
    #[diesel(sql_type = Text)]
    settings: String,
}

pub fn get(conn: &mut PgConnection, user_id: i32) -> DbResult<Option<String>> {
    Ok(diesel::sql_query("SELECT settings::text AS settings FROM user_stream_settings WHERE user_id=$1")
        .bind::<Integer, _>(user_id)
        .get_result::<SettingsRow>(conn)
        .optional()?
        .map(|row| row.settings))
}
