use crate::db::error::DbResult;
use crate::db::schema::streams;
use diesel::{
    prelude::*,
    sql_types::{Bool, Integer, Text},
};

#[derive(QueryableByName)]
struct State {
    #[diesel(sql_type = Bool)]
    live: bool,
}

/// Persist a successful SFU transition; retries of an already-live start do not notify twice.
pub fn stream_state(
    conn: &mut PgConnection,
    stream: i32,
    live: bool,
    host: &str,
    title: &str,
) -> DbResult<()> {
    conn.transaction(|conn| {
        streams::table.find(stream).select(streams::id).for_update().first::<i32>(conn)?;
        diesel::sql_query("INSERT INTO notification_stream_state (stream_id) VALUES ($1) ON CONFLICT DO NOTHING")
            .bind::<Integer,_>(stream).execute(conn)?;
        let previous: State = diesel::sql_query("SELECT live FROM notification_stream_state WHERE stream_id=$1 FOR UPDATE")
            .bind::<Integer,_>(stream).get_result(conn)?;
        if live {
            let open: State = diesel::sql_query("SELECT EXISTS (SELECT 1 FROM lobbies WHERE stream_id=$1 AND is_open) AS live")
                .bind::<Integer,_>(stream).get_result(conn)?;
            if !open.live { return Ok(()); }
        }
        if live && !previous.live {
            let event = uuid::Uuid::new_v4().to_string();
            let recipients = crate::db::schema::stream_friends::table
                .filter(crate::db::schema::stream_friends::stream_id.eq(stream))
                .filter(crate::db::schema::stream_friends::active.eq(true))
                .filter(crate::db::schema::stream_friends::accepted.eq(true))
                .select(crate::db::schema::stream_friends::user_id).load::<i32>(conn)?;
            for recipient in recipients {
                diesel::sql_query("INSERT INTO notifications (id,recipient_id,stream_id,event_key,title,body)
                    SELECT $1,$2,$3,$4,$5,$6 WHERE EXISTS (SELECT 1 FROM users WHERE id=$2 AND active) AND $2 <> (SELECT user_id FROM streams WHERE id=$3)
                    ON CONFLICT (recipient_id,event_key) DO NOTHING")
                    .bind::<Text,_>(uuid::Uuid::new_v4().to_string()).bind::<Integer,_>(recipient).bind::<Integer,_>(stream)
                    .bind::<Text,_>(&event).bind::<Text,_>("Stream started")
                    .bind::<Text,_>(format!("{host} started “{title}”. Join the stream."))
                    .execute(conn)?;
            }
        }
        diesel::sql_query("UPDATE notification_stream_state SET live=$2 WHERE stream_id=$1")
            .bind::<Integer,_>(stream).bind::<Bool,_>(live).execute(conn)?;
        Ok(())
    })
}
