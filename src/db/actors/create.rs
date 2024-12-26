use crate::db::actors::{Actor, ActorType};
use chrono::{NaiveDateTime, Utc};
use diesel::{Insertable, QueryResult, RunQueryDsl, Selectable, SelectableHelper, SqliteConnection};
use crate::util::iri::IriSet;
use crate::util::rsa::{KeyPair};

#[derive(Insertable, Selectable, Debug)]
#[diesel(table_name = crate::db::schema::actors)]
pub struct NewActor<'a> {
    pub preferred_username:  &'a str,
    pub actor_type:  &'a str,
    pub actor_iri:  &'a str,
    pub public_key:  &'a str,
    pub private_key:  &'a str,
    pub following_iri:  &'a str,
    pub followers_iri:  &'a str,
    pub inbox_iri:  &'a str,
    pub outbox_iri:  &'a str,
    pub shared_inbox_iri:  &'a str,
    pub instance_id: Option<i32>,
    pub created_at: NaiveDateTime,
}

pub fn upsert_new_instance_actor(
    conn: &mut SqliteConnection,
    inst_name: &str,
    domain: &str,
    tls: bool,
) -> QueryResult<Actor> {
    let actor = insert_new_actor(conn, inst_name, domain, ActorType::Application, tls, None, true)?;
    Ok(actor)
}

pub fn insert_new_person_actor(
    conn: &mut SqliteConnection,
    inst_name: &str,
    domain: &str,
    tls: bool,
    inst_id: i32,
) -> QueryResult<Actor> {
    let actor = insert_new_actor(conn, inst_name, domain, ActorType::Person, tls, Some(inst_id), false)?;
    Ok(actor)
}

pub fn insert_new_group_actor(
    conn: &mut SqliteConnection,
    inst_name: &str,
    domain: &str,
    tls: bool,
    inst_id: i32,
) -> QueryResult<Actor> {
    let actor = insert_new_actor(conn, inst_name, domain, ActorType::Group, tls, Some(inst_id), false)?;
    Ok(actor)
}

pub fn insert_new_actor(
    conn: &mut SqliteConnection,
    user_name:  &str,
    domain:  &str,
    type_of_actor: ActorType,
    tls: bool,
    id_of_instance: Option<i32>,
    upsert: bool,
) -> QueryResult<Actor> {
    let keys = KeyPair::new().map_err(|_| diesel::NotFound)?;
    let iri_set = IriSet::new(user_name, domain, tls);

    let new_actor = NewActor {
        preferred_username: user_name,
        actor_type: type_of_actor.val(),
        actor_iri: iri_set.actor.as_str(),
        public_key: keys.public_key.as_str(),
        private_key: keys.private_key.as_str(),
        following_iri: iri_set.following.as_str(),
        followers_iri: iri_set.followers.as_str(),
        inbox_iri: iri_set.inbox.as_str(),
        outbox_iri: iri_set.outbox.as_str(),
        shared_inbox_iri: iri_set.shared_inbox.as_str(),
        instance_id: id_of_instance,
        created_at: Utc::now().naive_utc(),
    };

    if upsert {
        use crate::db::schema::actors::dsl::*;
        let actor = diesel::insert_into(actors)
            .values(&new_actor)
            .on_conflict(actor_iri)
            .do_nothing()
            .get_result(conn)?;
        return Ok(actor)
    }

    use crate::db::schema::actors::dsl::*;
    let actor = diesel::insert_into(actors)
        .values(&new_actor)
        .returning(Actor::as_returning())
        .get_result::<Actor>(conn)?;
    Ok(actor)
}
