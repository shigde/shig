use crate::db::schema::actors;
use crate::db::schema::actors::dsl::*;
use crate::util::{iri, rsa};
use chrono::{NaiveDateTime, Utc};
use diesel::prelude::*;

#[derive(Queryable, Identifiable, Selectable, Debug, PartialEq)]
#[diesel(table_name = actors)]
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
    pub server_id: Option<i32>,
    pub remote_created_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Insertable, Debug)]
#[table_name = "actors"]
pub struct NewActor {
    pub preferred_username: String,
    pub actor_type: String,
    pub actor_iri: String,
    pub public_key: String,
    pub private_key: String,
    pub following_iri: String,
    pub followers_iri: String,
    pub inbox_iri: String,
    pub outbox_iri: String,
    pub shared_inbox_iri: String,
    pub created_at: NaiveDateTime,
}

fn insert_new_actor_by_user(
    conn: &mut SqliteConnection,
    user_name: String,
    domain: String,
    tls: bool,
) -> QueryResult<Actor> {
    let (prv_key, pub_key) = rsa::build_keys();
    let new_actor = NewActor {
        preferred_username: user_name.clone(),
        actor_type: "Person".to_string(),
        actor_iri: iri::build_actor_iri(user_name.as_str(), domain.as_str(), tls),
        public_key: format!("{:?}", pub_key),
        private_key: format!("{:?}", prv_key),
        following_iri: iri::build_following_iri(user_name.as_str(), domain.as_str(), tls),
        followers_iri: iri::build_followers_iri(user_name.as_str(), domain.as_str(), tls),
        inbox_iri: iri::build_inbox_iri(user_name.as_str(), domain.as_str(), tls),
        outbox_iri: iri::build_outbox_iri(user_name.as_str(), domain.as_str(), tls),
        shared_inbox_iri: iri::build_shared_inbox_iri(domain.as_str(), tls),
        created_at: Utc::now().naive_utc(),
    };

    diesel::insert_into(actors)
        .values(&new_actor)
        .execute(conn)?;

    let user = actors.first(conn)?;

    Ok(user)
}
