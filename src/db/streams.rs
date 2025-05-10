pub mod create;
pub mod delete;
pub mod read;
pub mod update;

use crate::db::channels::Channel;
use crate::db::users::User;
use chrono::NaiveDateTime;
use diesel::{Associations, Identifiable, Insertable, Queryable, Selectable};

#[derive(Queryable, Insertable, Identifiable, Selectable, Associations, Debug, PartialEq)]
#[diesel(belongs_to(User))]
#[diesel(belongs_to(Channel))]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = crate::db::schema::streams)]
pub struct Stream {
    pub id: i32,
    pub uuid: String,
    pub user_id: i32,
    pub channel_id: i32,
    pub title: String,
    pub description: Option<String>,
    pub support: Option<String>,
    pub date: NaiveDateTime,
    pub start_time: Option<NaiveDateTime>,
    pub end_time: Option<NaiveDateTime>,
    pub licence: i32,
    pub is_repeating: bool,
    pub repeat: Option<i32>,
    pub is_public: bool,
    pub is_live: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
