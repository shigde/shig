pub mod create;
pub mod delete;
pub mod read;
pub mod update;

use crate::db::channels::Channel;
use crate::db::friend_roles::FriendRole;
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
#[diesel(belongs_to(FriendRole))]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = crate::db::schema::channel_friends)]
pub struct ChannelFriend {
    pub id: i32,
    pub user_id: i32,
    pub channel_id: i32,
    pub friend_role_id: i32,
    pub active: bool,
    pub accepted: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
