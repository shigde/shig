use crate::db::instances::Instance;
use chrono::{NaiveDateTime, Utc};
use diesel::{
    Connection, Insertable, QueryResult, RunQueryDsl, SelectableHelper, SqliteConnection,
};
use crate::db::actors::create::upsert_new_instance_actor;

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
) -> QueryResult<Instance> {
    conn.transaction(move |conn| {
        let new_actor = upsert_new_instance_actor(conn, name, domain, tls)?;
        let new_instance = NewInstance {
            actor_id: new_actor.id,
            created_at: Utc::now().naive_utc(),
        };

        use crate::db::schema::instances::dsl::instances;
        let inst = diesel::insert_into(instances)
            .values(&new_instance)
            .returning(Instance::as_returning())
            .get_result(conn)?;

        Ok(inst)
    })
}
