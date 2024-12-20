use diesel::prelude::*;
use crate::db::schema::{user_roles};

#[derive(Queryable, Identifiable, Selectable, Debug, PartialEq)]
#[diesel(table_name = user_roles)]
pub struct UserRole {
    pub id: i32,
    pub name: String
}
