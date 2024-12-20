use diesel::prelude::*;
use crate::db::schema::{instances, actors};
use chrono::NaiveDateTime;

#[derive(Queryable, Identifiable, Selectable, Debug, PartialEq)]
#[diesel(table_name = instances)]
pub struct User {
    pub id: i32,
    pub actor_id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
