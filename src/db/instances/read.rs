use crate::db::actors::read::find_actor_by_actor_iri;
use crate::db::error::DbResult;
use crate::db::instances::Instance;
use crate::util::iri::build_actor_iri;
use diesel::{select, BelongingToDsl, EqAll, QueryDsl, RunQueryDsl, SelectableHelper, SqliteConnection};
use diesel::dsl::exists;

#[allow(dead_code)]
pub fn find_instance_by_domain(
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

pub fn exists_home_instance(conn: &mut SqliteConnection) -> DbResult<bool> {
    use crate::db::schema::instances::dsl::*;
    let exists = select(exists(instances.filter(is_home.eq_all(true)))).get_result(conn)?;
    Ok(exists)
}

pub fn find_home_instance(conn: &mut SqliteConnection) -> DbResult<Instance> {
    use crate::db::schema::instances;
    let inst = instances::table
        .filter(instances::is_home.eq_all(true))
        .select(Instance::as_select())
        .get_result(conn)?;

    Ok(inst)
}
