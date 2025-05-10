use crate::db::streams::Stream;
use chrono::NaiveDateTime;
use diesel::{Associations, Identifiable, QueryId, Queryable, QueryableByName, Selectable};

pub mod create;
pub mod delete;
pub mod read;
pub mod update;

#[derive(
    Queryable, QueryableByName, QueryId, Identifiable, Selectable, Associations, Debug, PartialEq,
)]
#[diesel(belongs_to(Stream))]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = crate::db::schema::stream_thumbnails)]
pub struct StreamThumbnail {
    pub id: i32,
    pub filename: String,
    pub height: Option<i32>,
    pub width: Option<i32>,
    pub file_url: Option<String>,
    pub on_disk: bool,
    pub stream_id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
