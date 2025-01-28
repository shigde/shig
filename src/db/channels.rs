pub mod create;
pub mod read;
pub mod delete;

use chrono::NaiveDateTime;
use diesel::{Associations, Identifiable, Insertable, Queryable, Selectable};
use crate::db::actors::Actor;
use crate::db::users::User;

#[derive(Queryable, Insertable, Identifiable, Selectable, Associations, Debug, PartialEq)]
#[diesel(belongs_to(User))]
#[diesel(belongs_to(Actor))]
#[diesel(table_name = crate::db::schema::channels)]
pub struct Channel {
    pub id: i32,
    pub user_id: i32,
    pub actor_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub support: Option<String>,
    pub public: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}
