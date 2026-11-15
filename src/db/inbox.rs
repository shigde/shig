pub mod message;
pub mod notification;
pub mod notification_stream_state;
pub mod read;
pub mod update;

use chrono::{DateTime, Utc};
use diesel::{
    sql_types::{BigInt, Nullable, Text, Timestamptz},
    QueryableByName,
};

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
#[derive(QueryableByName)]
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
#[derive(QueryableByName)]
pub struct Counts {
    #[diesel(sql_type = BigInt)]
    pub total: i64,
    #[diesel(sql_type = BigInt)]
    pub unread: i64,
}
pub struct Page {
    pub items: Vec<Entry>,
    pub total: i64,
    pub unread: i64,
}
