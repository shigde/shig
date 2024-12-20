use diesel::prelude::*;
use crate::db::schema::{instances};
use crate::db::actors::{Actor};
use chrono::NaiveDateTime;

#[derive(Queryable, Identifiable, Selectable, Associations, Debug, PartialEq)]
#[diesel(belongs_to(Actor))]
#[diesel(table_name = instances)]
pub struct User {
    pub id: i32,
    pub actor_id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
