pub mod create;

pub struct Notification<'a> {
    pub id: String,
    pub recipient_id: i32,
    pub stream_id: Option<i32>,
    pub action_path: Option<&'a str>,
    pub event_key: &'a str,
    pub title: &'a str,
    pub body: &'a str,
}
