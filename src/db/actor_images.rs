pub mod create;
pub mod delete;
pub mod read;
pub mod update;

use crate::db::actors::Actor;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{
    Associations, Identifiable, Insertable, QueryId, Queryable, QueryableByName, Selectable,
};

#[derive(
    Queryable, QueryableByName, QueryId, Identifiable, Selectable, Associations, Debug, PartialEq,
)]
#[diesel(belongs_to(Actor))]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = crate::db::schema::actor_images)]
pub struct ActorImage {
    pub id: i32,
    pub filename: String,
    pub height: Option<i32>,
    pub width: Option<i32>,
    pub file_url: Option<String>,
    pub on_disk: bool,
    pub image_type: String,
    pub actor_id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[allow(dead_code)]
pub enum ActorImageType {
    ACTOR,
    BANNER,
    THUMBNAIL,
}

impl ActorImageType {
    pub fn value_as_str(&self) -> &str {
        match &self {
            ActorImageType::ACTOR => "actor",
            ActorImageType::BANNER => "banner",
            ActorImageType::THUMBNAIL => "thumbnail",
            _ => "thumbnail",
        }
    }
}
