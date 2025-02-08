pub mod create;
pub mod read;

use crate::db::actors::Actor;
use chrono::NaiveDateTime;
use diesel::prelude::*;

#[derive(Queryable, Identifiable, Selectable, Associations, Debug, PartialEq)]
#[diesel(belongs_to(Actor))]
#[diesel(table_name = crate::db::schema::instances)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Instance {
    pub id: i32,
    pub actor_id: i32,
    pub is_home: bool,
    pub domain: String,
    pub tls: bool,
    pub token: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl Instance {
    pub fn get_base_url(&self) -> String {
        let http = if self.tls { "https" } else { "http" };
        format!("{}://{}", http, self.domain)
    }
}
