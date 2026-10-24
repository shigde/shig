use crate::db::error::DbResult;
use diesel::{
    prelude::*,
    sql_types::{Integer, Text},
};

pub fn invitation(
    conn: &mut PgConnection,
    recipient: i32,
    stream: i32,
    invitation: i32,
    host: &str,
    title: &str,
) -> DbResult<()> {
    diesel::sql_query("INSERT INTO messages (id, recipient_id, stream_id, event_key, title, body) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (recipient_id, event_key) DO NOTHING")
        .bind::<Text,_>(uuid::Uuid::new_v4().to_string()).bind::<Integer,_>(recipient).bind::<Integer,_>(stream)
        .bind::<Text,_>(format!("invitation:{invitation}"))
        .bind::<Text,_>("Stream invitation")
        .bind::<Text,_>(format!("{host} added you as a guest to “{title}”."))
        .execute(conn)?;
    Ok(())
}
