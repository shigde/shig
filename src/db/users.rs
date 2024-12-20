use diesel::prelude::*;
use crate::db::schema::{users, actors};
use chrono::NaiveDateTime;

#[derive(Queryable, Identifiable, Selectable, Debug, PartialEq)]
#[diesel(table_name = users)]
pub struct User {
    pub id: i32,
    pub uuid: String,
    pub name: String,
    pub email: String,
    pub password: String,
    pub role: i32,
    pub active: bool,
    pub actor_id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
