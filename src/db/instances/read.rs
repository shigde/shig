use crate::db::actors::read::find_actor_by_actor_iri;
use crate::db::instances::Instance;
use crate::util::iri::build_actor_iri;
use diesel::{
    BelongingToDsl, QueryDsl, QueryResult, RunQueryDsl, SelectableHelper, SqliteConnection,
};

pub fn find_instance_by_actor(
    conn: &mut SqliteConnection,
    name: &str,
    domain: &str,
    tls: bool,
) -> QueryResult<Instance> {
    let actor_iri = build_actor_iri(name, domain, tls);
    let actor = find_actor_by_actor_iri(conn, &actor_iri[..])?;

    let inst = Instance::belonging_to(&actor)
        .select(Instance::as_select())
        .first(conn)?;

    Ok(inst)
}
