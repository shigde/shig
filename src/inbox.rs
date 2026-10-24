//! Shared persistence for the two recipient-owned inboxes. Table names are internal constants.
use crate::db::error::DbResult;
use chrono::{DateTime, Utc};
use diesel::{
    prelude::*,
    sql_types::{BigInt, Integer, Nullable, Text, Timestamptz},
};
use serde::Serialize;

#[derive(Clone, Copy)]
pub enum Inbox {
    Messages,
    Notifications,
}
impl Inbox {
    fn table(self) -> &'static str {
        match self {
            Self::Messages => "messages",
            Self::Notifications => "notifications",
        }
    }
}
#[derive(QueryableByName, Serialize)]
pub struct Entry {
    #[diesel(sql_type = Text)]
    pub id: String,
    #[diesel(sql_type = Text)]
    pub title: String,
    #[diesel(sql_type = Text)]
    pub body: String,
    #[diesel(sql_type = Timestamptz)]
    pub created_at: DateTime<Utc>,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    pub read_at: Option<DateTime<Utc>>,
    #[diesel(sql_type = Nullable<Text>)]
    pub action_path: Option<String>,
}
#[derive(QueryableByName, Serialize)]
pub struct Counts {
    #[diesel(sql_type = BigInt)]
    pub total: i64,
    #[diesel(sql_type = BigInt)]
    pub unread: i64,
}
#[derive(Serialize)]
pub struct Page {
    pub items: Vec<Entry>,
    pub total: i64,
    pub unread: i64,
}

pub fn counts(conn: &mut PgConnection, inbox: Inbox, user: i32) -> DbResult<Counts> {
    Ok(diesel::sql_query(format!("SELECT count(*) AS total, count(*) FILTER (WHERE read_at IS NULL) AS unread FROM {} WHERE recipient_id = $1", inbox.table()))
        .bind::<Integer,_>(user).get_result(conn)?)
}
pub fn list(conn: &mut PgConnection, inbox: Inbox, user: i32, offset: i64) -> DbResult<Page> {
    let items = diesel::sql_query(format!("SELECT e.id::text, e.title, e.body, e.created_at, e.read_at,
        CASE WHEN s.id IS NOT NULL THEN '/channel/' || c.uuid || '/stream/' || s.uuid || '/lobby' ELSE NULL END AS action_path
        FROM {} e LEFT JOIN streams s ON s.id = e.stream_id LEFT JOIN channels c ON c.id = s.channel_id
        WHERE e.recipient_id = $1 ORDER BY e.created_at DESC, e.id DESC LIMIT 25 OFFSET $2", inbox.table()))
        .bind::<Integer,_>(user).bind::<BigInt,_>(offset).load(conn)?;
    let counts = counts(conn, inbox, user)?;
    Ok(Page {
        items,
        total: counts.total,
        unread: counts.unread,
    })
}
pub fn mark_read(conn: &mut PgConnection, inbox: Inbox, user: i32, id: &str) -> DbResult<bool> {
    Ok(diesel::sql_query(format!(
        "UPDATE {} SET read_at = COALESCE(read_at, now()) WHERE recipient_id = $1 AND id = $2",
        inbox.table()
    ))
    .bind::<Integer, _>(user)
    .bind::<Text, _>(id)
    .execute(conn)?
        > 0)
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
