pub mod create;
pub mod delete;
pub mod read;
pub mod update;

use crate::db::friend_roles::FriendRole;
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
#[diesel(belongs_to(Stream))]
#[diesel(belongs_to(FriendRole))]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = crate::db::schema::stream_friends)]
pub struct StreamFriend {
    pub id: i32,
    pub user_id: i32,
    pub stream_id: i32,
    pub friend_role_id: i32,
    pub active: bool,
    pub accepted: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
