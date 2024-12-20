use diesel::prelude::*;
use crate::db::schema::{actors};
use chrono::NaiveDateTime;

#[derive(Queryable, Identifiable, Selectable, Debug, PartialEq)]
#[diesel(table_name = actors)]
pub struct User {
    pub id: i32,
    pub preferred_username:String,
    pub actor_type: String,
    pub actor_iri: String,
    pub public_key: String,
    pub private_key: String,
    pub following_iri:  String,
    pub followers_iri:  String,
    pub inbox_iri: String,
    pub outbox_iri: String,
    pub shared_inbox_iri: String,
    pub remote_created_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
