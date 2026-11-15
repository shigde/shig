pub mod create;
pub mod delete;
pub mod read;

use crate::db::{streams::Stream, users::User};
use chrono::NaiveDateTime;
use diesel::{Associations, Identifiable, Queryable, Selectable};

#[derive(Associations, Identifiable, Queryable, Selectable, Debug, PartialEq)]
#[diesel(belongs_to(Stream))]
#[diesel(belongs_to(User))]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = crate::db::schema::stream_likes)]
pub struct StreamLike {
    pub id: i32,
    pub stream_id: i32,
    pub user_id: i32,
    pub created_at: NaiveDateTime,
}
