pub mod create;
pub mod read;
pub mod update;

use crate::db::channels::Channel;
use crate::db::streams::Stream;
use crate::db::users::User;
use chrono::NaiveDateTime;
use diesel::{Associations, Identifiable, Insertable, QueryId, Queryable, Selectable};
use serde::Serialize;

#[derive(
    Queryable,
    QueryId,
    Insertable,
    Identifiable,
    Selectable,
    Associations,
    Debug,
    PartialEq,
    Serialize,
)]
#[diesel(belongs_to(User))]
#[diesel(belongs_to(Channel))]
#[diesel(belongs_to(Stream))]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = crate::db::schema::lobbies)]
pub struct Lobby {
    pub id: i32,
    pub uuid: String,
    pub user_id: i32,
    pub channel_id: i32,
    pub stream_id: Option<i32>,
    pub secret: String,
    pub is_open: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
