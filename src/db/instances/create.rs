use crate::db::instances::Instance;
use chrono::{NaiveDateTime, Utc};
use diesel::{
    Connection, Insertable, RunQueryDsl, SelectableHelper, SqliteConnection,
};
use crate::db::actors::create::insert_new_instance_actor;
use crate::db::actors::read::exists_instance_actor;
use crate::db::error::DbResult;
use crate::db::instances::read::find_instance_by_actor_iri;
use crate::util::iri::build_actor_iri;

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::db::schema::instances)]
pub struct NewInstance {
    pub actor_id: i32,
    pub created_at: NaiveDateTime,
}

pub fn upsert_new_instance(
    conn: &mut SqliteConnection,
    name: &str,
    domain: &str,
    tls: bool,
) -> DbResult<Instance> {
    conn.transaction(move |conn| {
        let iri = build_actor_iri(name, domain, tls);
        let  exists=  exists_instance_actor(conn, iri.as_str())
            .map_err(|e| -> String { format!("checking existing instance: {}", e) })?;

        if exists {
            let current_inst = find_instance_by_actor_iri(conn, iri.as_str())
                .map_err(|e| -> String { format!("reading existing instance: {}", e) })?;

            return Ok(current_inst)
        }


        let new_actor = insert_new_instance_actor(conn, name, domain, tls)
            .map_err(|e| -> String { format!("insert new inst actor: {}", e) })?;

        let new_instance = NewInstance {
            actor_id: new_actor.id,
            created_at: Utc::now().naive_utc(),
        };

        use crate::db::schema::instances::dsl::instances;
        let inst = diesel::insert_into(instances)
            .values(&new_instance)
            .returning(Instance::as_returning())
            .get_result(conn)
            .map_err(|e| -> String { format!("query instance: {}", e) })?;

        Ok(inst)
    })
}
