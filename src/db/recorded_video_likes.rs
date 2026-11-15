pub mod create;
pub mod delete;
pub mod read;

use chrono::NaiveDateTime;
use diesel::{Identifiable, Queryable, Selectable};

#[derive(Identifiable, Queryable, Selectable, Debug, PartialEq)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = crate::db::schema::recorded_video_likes)]
pub struct RecordedVideoLike {
    pub id: i32,
    pub recorded_video_id: i32,
    pub user_id: i32,
    pub created_at: NaiveDateTime,
}
