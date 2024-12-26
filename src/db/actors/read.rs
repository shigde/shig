use crate::db::actors::Actor;
use crate::db::schema::actors;
use diesel::{EqAll, QueryDsl, QueryResult, RunQueryDsl, SelectableHelper, SqliteConnection};

pub fn find_actor_by_actor_iri(conn: &mut SqliteConnection, actor_iri: &str) -> QueryResult<Actor> {
    let actor = actors::table
        .filter(actors::actor_iri.eq_all(actor_iri))
        .select(Actor::as_select())
        .get_result(conn)?;

    Ok(actor)
}
