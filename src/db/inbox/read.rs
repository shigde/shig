use super::{Counts, Inbox, Page};
use crate::db::error::DbResult;
use diesel::{
    prelude::*,
    sql_types::{BigInt, Integer},
};

pub fn counts(conn: &mut PgConnection, inbox: Inbox, user: i32) -> DbResult<Counts> {
    let query = format!(
        "SELECT count(*) AS total, count(*) FILTER (WHERE read_at IS NULL) AS unread
         FROM {} WHERE recipient_id = $1",
        inbox.table()
    );
    Ok(diesel::sql_query(query)
        .bind::<Integer, _>(user)
        .get_result(conn)?)
}

pub fn list(conn: &mut PgConnection, inbox: Inbox, user: i32, offset: i64) -> DbResult<Page> {
    let query = format!(
        "SELECT e.id::text, e.title, e.body, e.created_at, e.read_at,
            CASE WHEN s.id IS NOT NULL THEN
                '/channel/' || c.uuid || '/stream/' || s.uuid || '/lobby'
            ELSE NULL END AS action_path
         FROM {} e
         LEFT JOIN streams s ON s.id = e.stream_id
         LEFT JOIN channels c ON c.id = s.channel_id
         WHERE e.recipient_id = $1
         ORDER BY e.created_at DESC, e.id DESC
         LIMIT 25 OFFSET $2",
        inbox.table()
    );
    let items = diesel::sql_query(query)
        .bind::<Integer, _>(user)
        .bind::<BigInt, _>(offset)
        .load(conn)?;
    let counts = counts(conn, inbox, user)?;
    Ok(Page {
        items,
        total: counts.total,
        unread: counts.unread,
    })
}
