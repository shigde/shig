use diesel::prelude::*;
use crate::db::schema::{users};
use crate::db::actors::{Actor};
use crate::db::user_roles::{UserRole};
use chrono::NaiveDateTime;

#[derive(Queryable, Identifiable, Selectable, Associations, Debug, PartialEq)]
#[diesel(belongs_to(Actor))]
#[diesel(belongs_to(UserRole))]
#[diesel(table_name = users)]
pub struct User {
    pub id: i32,
    pub uuid: String,
    pub name: String,
    pub email: String,
    pub password: String,
    pub user_role_id: i32,
    pub active: bool,
    pub actor_id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
