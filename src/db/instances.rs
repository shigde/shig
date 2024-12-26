pub mod create;
mod read;

use crate::db::actors::Actor;
use crate::db::schema::instances;
use chrono::{NaiveDateTime};
use diesel::prelude::*;

#[derive(Queryable, Identifiable, Selectable, Associations, Debug, PartialEq)]
#[diesel(belongs_to(Actor))]
#[diesel(table_name = instances)]
pub struct Instance {
    pub id: i32,
    pub actor_id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

