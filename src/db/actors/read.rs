use crate::db::actors::Actor;
use crate::db::error::DbResult;
use diesel::dsl::{exists, select};
use diesel::{QueryDsl, RunQueryDsl, SelectableHelper, PgConnection};
use diesel::prelude::*;

pub fn find_actor_by_actor_iri(conn: &mut PgConnection, iri: &str) -> DbResult<Actor> {
    use crate::db::schema::actors;
    let actor = actors::table
        .filter(actors::actor_iri.eq(iri))
        .select(Actor::as_select())
        .get_result(conn)?;

    Ok(actor)
}

pub fn exists_actor(conn: &mut PgConnection, iri: &str) -> DbResult<bool> {
    use crate::db::schema::actors::dsl::*;
    let exists = select(exists(actors.filter(actor_iri.eq(iri)))).get_result(conn)?;
    Ok(exists)
}
