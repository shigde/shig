pub mod create;

use crate::db::schema::messages;
use diesel::Insertable;

#[derive(Insertable)]
#[diesel(table_name = messages)]
pub struct Message<'a> {
    pub id: String,
    pub recipient_id: i32,
    pub stream_id: i32,
    pub event_key: String,
    pub title: &'a str,
    pub body: String,
}
