pub mod create;
pub mod delete;
pub mod read;
pub mod update;

use crate::db::actors::Actor;
use crate::db::error::DbResult;
use crate::db::users::read::find_user_by_uuid;
use crate::db::users::User;
use chrono::NaiveDateTime;
use diesel::{Associations, Identifiable, Insertable, PgConnection, Queryable, Selectable};

#[derive(Queryable, Insertable, Identifiable, Selectable, Associations, Debug, PartialEq)]
#[diesel(belongs_to(User))]
#[diesel(belongs_to(Actor))]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = crate::db::schema::channels)]
pub struct Channel {
    pub id: i32,
    pub uuid: String,
    pub user_id: i32,
    pub actor_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub support: Option<String>,
    pub public: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl Channel {
    pub fn from_model(conn: &mut PgConnection, uuid: String) -> DbResult<User> {
        find_user_by_uuid(conn, uuid)
    }
}
