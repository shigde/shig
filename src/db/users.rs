pub mod create;
pub mod read;

use crate::db::actors::Actor;
use bcrypt::verify;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use crate::db::error::DbResult;
use crate::db::users::read::find_user_by_uuid;

#[derive(Queryable, Insertable, Identifiable, Selectable, Associations, Debug, PartialEq)]
#[diesel(belongs_to(Actor))]
#[diesel(table_name = crate::db::schema::users)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub user_uuid: String,
    pub user_role_id: i32,
    pub password: String,
    pub active: bool,
    pub actor_id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

impl User {
    pub fn verify(&self, password: String) -> bool {
        verify(password.as_str(), &self.password).unwrap_or_else(|_| false)
    }

    pub fn from_uuid(conn: &mut SqliteConnection, uuid: String) -> DbResult<User> {
        find_user_by_uuid(conn, uuid)
    }
}
