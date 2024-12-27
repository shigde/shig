use crate::db::actors::read::find_actor_by_actor_iri;
use crate::db::error::DbResult;
use crate::db::instances::Instance;
use crate::util::iri::build_actor_iri;
use diesel::{BelongingToDsl, QueryDsl, RunQueryDsl, SelectableHelper, SqliteConnection};

#[allow(dead_code)]
pub fn find_instance_by_actor(
    conn: &mut SqliteConnection,
    name: &str,
    domain: &str,
    tls: bool,
) -> DbResult<Instance> {
    let actor_iri = build_actor_iri(name, domain, tls);
    let inst = find_instance_by_actor_iri(conn, actor_iri.as_str())?;

    Ok(inst)
}

pub fn find_instance_by_actor_iri(conn: &mut SqliteConnection, iri: &str) -> DbResult<Instance> {
    let actor = find_actor_by_actor_iri(conn, iri)?;

    let inst = Instance::belonging_to(&actor)
        .select(Instance::as_select())
        .first(conn)?;

    Ok(inst)
}
