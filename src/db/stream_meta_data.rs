pub mod create;
pub mod update;

use crate::db::streams::Stream;
use chrono::NaiveDateTime;
use diesel::{Associations, Identifiable, Insertable, Queryable, Selectable};
use serde::Serialize;

#[derive(Queryable, Insertable, Identifiable, Selectable, Associations, Debug, PartialEq, Serialize, Clone)]
#[diesel(belongs_to(Stream))]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = crate::db::schema::stream_meta_data)]
pub struct StreamMetaData {
    pub id: i32,
    pub stream_id: i32,
    pub is_shig: bool,
    pub stream_key: String,
    pub url: String,
    pub protocol: i32,
    pub permanent_live: bool,
    pub save_replay: bool,
    pub latency_mode: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
