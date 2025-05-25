pub mod read;

use chrono::NaiveDateTime;
use diesel::{Identifiable, Insertable, Queryable, Selectable};

#[derive(Queryable, Insertable, Identifiable, Selectable, Debug, PartialEq)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = crate::db::schema_views::stream_previews)]
pub struct StreamPreview {
    pub id: i32,
    pub title: String,
    pub thumbnail: Option<String>,
    pub uuid: String,
    pub description: Option<String>,
    pub support: Option<String>,
    pub date: NaiveDateTime,
    pub start_time: Option<NaiveDateTime>,
    pub end_time: Option<NaiveDateTime>,
    pub is_live: bool,
    pub is_public: bool,
    pub owner_name: String,
    pub owner_uuid: String,
    pub owner_avatar: Option<String>,
    pub channel_name: String,
    pub channel_uuid: String,
}
