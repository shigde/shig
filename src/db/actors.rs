pub mod create;
pub mod read;

use chrono::NaiveDateTime;
use diesel::prelude::*;

#[derive(Queryable, Identifiable, Selectable, Debug, PartialEq)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(table_name = crate::db::schema::actors)]
pub struct Actor {
    pub id: i32,
    pub preferred_username: String,
    pub actor_type: String,
    pub actor_iri: String,
    pub public_key: Option<String>,
    pub private_key: Option<String>,
    pub following_iri: Option<String>,
    pub followers_iri: Option<String>,
    pub inbox_iri: Option<String>,
    pub outbox_iri: Option<String>,
    pub shared_inbox_iri: Option<String>,
    pub instance_id: Option<i32>,
    pub remote_created_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

pub enum ActorType {
    Person,
    Group,
    Application,
    Service,
}

impl ActorType {
    pub fn val(&self) -> &str {
        match self {
            Self::Person => "Person",
            Self::Group => "Group",
            Self::Application => "Application",
            Self::Service => "Service",
        }

    }
}
