use crate::db::actors::Actor;
use crate::db::error::DbResult;
use diesel::dsl::{exists, select};
use diesel::{QueryDsl, QueryResult, RunQueryDsl, SelectableHelper, SqliteConnection};
use diesel::prelude::*;

pub fn find_actor_by_actor_iri(conn: &mut SqliteConnection, iri: &str) -> QueryResult<Actor> {
    use crate::db::schema::actors;
    let actor = actors::table
        .filter(actors::actor_iri.eq(iri))
        .select(Actor::as_select())
        .get_result(conn)?;

    Ok(actor)
}

pub fn exists_actor(conn: &mut SqliteConnection, iri: &str) -> DbResult<bool> {
    use crate::db::schema::actors::dsl::*;
    let exists = select(exists(actors.filter(actor_iri.eq(iri)))).get_result(conn)?;
    Ok(exists)
}
