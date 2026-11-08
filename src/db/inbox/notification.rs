pub mod create;

pub struct Notification<'a> {
    pub id: String,
    pub recipient_id: i32,
    pub stream_id: i32,
    pub event_key: &'a str,
    pub title: &'a str,
    pub body: &'a str,
}
