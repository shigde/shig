use crate::db::actors::create::{insert_new_group_actor, insert_new_person_actor};
use crate::db::actors::read::exists_actor;
use crate::db::error::DbResult;
use crate::db::instances::read::find_home_instance;
use crate::db::user_roles::Role;
use crate::db::users::read::find_user_by_actor_iri;
use crate::db::users::User;

use crate::db::channels::create::insert_new_channel;
use crate::db::instances::Instance;
use crate::util::domain::{build_default_channel_name, build_domain_name};
use crate::util::iri::build_actor_iri;
use bcrypt::{hash, DEFAULT_COST};
use chrono::{NaiveDateTime, Utc};
use diesel::{Connection, Insertable, RunQueryDsl, SelectableHelper, SqliteConnection};
use uuid::Uuid;

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::db::schema::users)]
pub struct NewUser<'a> {
    pub user_uuid: &'a str,
    pub name: &'a str,
    pub email: &'a str,
    pub user_role_id: i32,
    pub password: &'a str,
    pub active: bool,
    pub actor_id: i32,
    pub created_at: NaiveDateTime,
}

pub fn create_new_user(
    conn: &mut SqliteConnection,
    user_name: &str,
    user_email: &str,
    user_pass: &str,
    user_role: Role,
    domain: &str,
    tls: bool,
) -> DbResult<User> {
    conn.transaction(move |conn| {
        let iri = build_actor_iri(user_name, domain, tls);

        // Check actor already exists
        let actor_exists = exists_actor(conn, iri.as_str())
            .map_err(|e| -> String { format!("checking user actor: {}", e) })?;

        if actor_exists {
            let current_user = find_user_by_actor_iri(conn, iri.as_str())
                .map_err(|e| -> String { format!("reading existing user: {}", e) })?;

            return Ok(current_user);
        }

        // Read home instance
        let inst: Instance =
            find_home_instance(conn).map_err(|e| -> String { format!("read instance: {}", e) })?;

        // Create a user actor
        let new_user_actor = insert_new_person_actor(conn, user_name, domain, tls, inst.id)
            .map_err(|e| -> String { format!("insert new user actor: {}", e) })?;

        // Create a user
        let user_domain_name = build_domain_name(user_name, domain);
        let user = insert_new_user(
            conn,
            user_domain_name.as_str(),
            user_email,
            user_pass,
            user_role,
            new_user_actor.id,
        )
        .map_err(|e| -> String { format!("insert new user: {}", e) })?;

        // Create a group actor
        let channel_name = build_default_channel_name(user_name);
        let new_group_actor = insert_new_group_actor(conn, channel_name.as_str(), domain, tls, inst.id)
            .map_err(|e| -> String { format!("insert new group actor: {}", e) })?;

        // Create a default channel for new users
        let channel_domain_name = build_domain_name(channel_name.as_str(), domain);
        let _ = insert_new_channel(
            conn,
            channel_domain_name.as_str(),
            user.id,
            new_group_actor.id,
        )
        .map_err(|e| -> String { format!("insert new group: {}", e) })?;

        Ok(user)
    })
}

pub fn insert_new_user(
    conn: &mut SqliteConnection,
    user_name: &str,
    user_email: &str,
    user_pass: &str,
    user_role: Role,
    actor_id: i32,
) -> DbResult<User> {
    let hashed_pass: String = hash(user_pass, DEFAULT_COST).unwrap();
    let uid = Uuid::new_v4().to_string();

    let new_user = NewUser {
        user_uuid: &uid,
        name: user_name,
        email: user_email,
        password: hashed_pass.as_str(),
        user_role_id: user_role.val(),
        active: true,
        actor_id,
        created_at: Utc::now().naive_utc(),
    };

    use crate::db::schema::users::dsl::users;
    let user = diesel::insert_into(users)
        .values(&new_user)
        .returning(User::as_returning())
        .get_result::<User>(conn)
        .map_err(|e| -> String { format!("create user: {}", e) })?;

    Ok(user)
}
