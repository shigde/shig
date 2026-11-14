use crate::db::{
    error::DbResult,
    inbox::{
        self as db, message::Message, notification::Notification,
        notification_stream_state as stream_state_db,
    },
    lobbies, stream_friends, streams,
};
use chrono::{DateTime, Utc};
use diesel::{Connection, PgConnection};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct Entry {
    id: String,
    title: String,
    body: String,
    created_at: DateTime<Utc>,
    read_at: Option<DateTime<Utc>>,
    action_path: Option<String>,
}

impl From<db::Entry> for Entry {
    fn from(entry: db::Entry) -> Self {
        Self {
            id: entry.id,
            title: entry.title,
            body: entry.body,
            created_at: entry.created_at,
            read_at: entry.read_at,
            action_path: entry.action_path,
        }
    }
}

#[derive(Serialize)]
pub struct Counts {
    total: i64,
    unread: i64,
}

impl From<db::Counts> for Counts {
    fn from(counts: db::Counts) -> Self {
        Self {
            total: counts.total,
            unread: counts.unread,
        }
    }
}

#[derive(Serialize)]
pub struct Page {
    items: Vec<Entry>,
    total: i64,
    unread: i64,
}

impl From<db::Page> for Page {
    fn from(page: db::Page) -> Self {
        Self {
            items: page.items.into_iter().map(Entry::from).collect(),
            total: page.total,
            unread: page.unread,
        }
    }
}

pub fn invitation(
    conn: &mut PgConnection,
    recipient: i32,
    stream: i32,
    invitation: i32,
    host: &str,
    title: &str,
) -> DbResult<()> {
    db::message::create::insert(
        conn,
        &Message {
            id: Uuid::new_v4().to_string(),
            recipient_id: recipient,
            stream_id: stream,
            event_key: format!("invitation:{invitation}"),
            title: "Stream invitation",
            body: format!("{host} added you as a guest to “{title}”."),
        },
    )
}

/// Notify accepted guests once for each successful transition to live.
pub fn stream_state(
    conn: &mut PgConnection,
    stream: i32,
    live: bool,
    host: &str,
    title: &str,
) -> DbResult<()> {
    conn.transaction(|conn| {
        streams::read::lock_stream(conn, stream)?;
        stream_state_db::create::ensure(conn, stream)?;
        let previous = stream_state_db::read::lock(conn, stream)?;
        if live && !lobbies::read::is_open_for_stream(conn, stream)? {
            return Ok(());
        }
        if live && !previous.live {
            let event_key = Uuid::new_v4().to_string();
            let body = format!("{host} started “{title}”. Join the stream.");
            for recipient in stream_friends::read::list_active_accepted_user_ids(conn, stream)? {
                db::notification::create::insert_for_active_recipient(
                    conn,
                    &Notification {
                        id: Uuid::new_v4().to_string(),
                        recipient_id: recipient,
                        stream_id: Some(stream),
                        action_path: None,
                        event_key: &event_key,
                        title: "Stream started",
                        body: &body,
                    },
                )?;
            }
        }
        stream_state_db::update::set_live(conn, stream, live)
    })
}

pub fn streaming_permission_requested(
    conn: &mut PgConnection,
    requester_id: i32,
    requester_name: &str,
    channel_uuid: &str,
) -> DbResult<()> {
    let event_key = format!("streaming-permission-request:{channel_uuid}:{}", Uuid::new_v4());
    let body = format!("{requester_name} requested permission to stream on channel {channel_uuid}.");
    db::notification::create::insert_for_active_admins(
        conn,
        &Uuid::new_v4().to_string(),
        requester_id,
        &event_key,
        "Streaming permission requested",
        &body,
        "/admin/users",
    )
}

pub fn streaming_permission_changed(
    conn: &mut PgConnection,
    recipient_id: i32,
    title: &str,
    body: &str,
    channel_uuid: &str,
) -> DbResult<()> {
    let event_key = format!("streaming-permission:{}", Uuid::new_v4());
    db::notification::create::insert_for_active_recipient(
        conn,
        &Notification {
            id: Uuid::new_v4().to_string(),
            recipient_id,
            stream_id: None,
            action_path: Some(&format!("/channel/{channel_uuid}?streamingPermissionRequired=true")),
            event_key: &event_key,
            title,
            body,
        },
    )
}
