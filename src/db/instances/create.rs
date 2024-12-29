use bcrypt::{hash, DEFAULT_COST};
use crate::db::actors::create::insert_new_instance_actor;
use crate::db::actors::read::exists_actor;
use crate::db::error::{DbResult};
use crate::db::instances::read::{exists_home_instance, find_home_instance, find_instance_by_actor_iri};
use crate::db::instances::Instance;
use crate::util::iri::build_actor_iri;
use chrono::{NaiveDateTime, Utc};
use diesel::{Connection, Insertable, RunQueryDsl, SelectableHelper, SqliteConnection};

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::db::schema::instances)]
pub struct NewInstance {
    pub actor_id: i32,
    pub is_home: bool,
    pub domain: String,
    pub tls: bool,
    pub token: Option<String>,
    pub created_at: NaiveDateTime,
}

pub fn upsert_new_instance(
    conn: &mut SqliteConnection,
    name: &str,
    home: bool,
    domain: &str,
    tls: bool,
    token_opt: Option<&str>,
) -> DbResult<Instance> {
    conn.transaction(move |conn| {

        // check home instance
        if home == true {
            let home_exists = exists_home_instance(conn)
                .map_err(|e| -> String { format!("checking home instance: {}", e) })?;

            if home_exists {
                log::info!("The home instance already exists and will not be updated!");
                let home_inst = find_home_instance(conn)
                    .map_err(|e| -> String { format!("fetching home instance: {}", e) })?;
                return Ok(home_inst);
            }
        }

        // check an actor already exists
        let iri = build_actor_iri(name, domain, tls);
        let exists = exists_actor(conn, iri.as_str())
            .map_err(|e| -> String { format!("checking instance actor: {}", e) })?;

        if exists {
            let current_inst = find_instance_by_actor_iri(conn, iri.as_str())
                .map_err(|e| -> String { format!("reading existing instance: {}", e) })?;

            return Ok(current_inst);
        }

        // create instance with an actor
        let new_actor = insert_new_instance_actor(conn, name, domain, tls)
            .map_err(|e| -> String { format!("insert new inst actor: {}", e) })?;

        let token = match token_opt {
            Some(origin) => {
                let hashed = hash(origin, DEFAULT_COST).unwrap();
                Some(hashed)
            },
            None => None
        };

        let new_instance = NewInstance {
            actor_id: new_actor.id,
            is_home: home,
            domain: String::from(domain),
            tls,
            token,
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
