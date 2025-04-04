use crate::db::actors::read::find_actor_by_actor_iri;
use crate::db::error::DbResult;
use crate::db::users::User;
use diesel::dsl::exists;
use diesel::prelude::*;

use diesel::{select, BelongingToDsl, PgConnection, QueryDsl, RunQueryDsl, SelectableHelper};

pub fn find_user_by_actor_iri(conn: &mut PgConnection, iri: &str) -> DbResult<User> {
    let actor = find_actor_by_actor_iri(conn, iri)?;
    let user = User::belonging_to(&actor)
        .select(User::as_select())
        .first(conn)?;

    Ok(user)
}

pub fn find_user_by_uuid(conn: &mut PgConnection, needle_uuid: String) -> DbResult<User> {
    use crate::db::schema::users::dsl::users;
    use crate::db::schema::users::user_uuid;
    let user = users
        .filter(user_uuid.eq(needle_uuid))
        .select(User::as_select())
        .first(conn)?;

    Ok(user)
}

pub fn find_user_by_email(conn: &mut PgConnection, needle_email: String) -> DbResult<User> {
    use crate::db::schema::users::dsl::users;
    use crate::db::schema::users::email;
    let user = users
        .filter(email.eq(needle_email))
        .select(User::as_select())
        .first(conn)?;

    Ok(user)
}

pub fn find_user_by_id(conn: &mut PgConnection, needle_id: i32) -> DbResult<User> {
    use crate::db::schema::users::dsl::users;
    use crate::db::schema::users::id;
    let user = users
        .filter(id.eq(needle_id))
        .select(User::as_select())
        .first(conn)?;

    Ok(user)
}

pub fn exists_user_by_email(conn: &mut PgConnection, user_email: &str) -> DbResult<bool> {
    use crate::db::schema::users::dsl::*;
    let exists = select(exists(users.filter(email.eq(user_email)))).get_result(conn)?;
    Ok(exists)
}

pub fn is_active_user_by_email(conn: &mut PgConnection, user_email: &str) -> DbResult<bool> {
    use crate::db::schema::users::dsl::*;
    let exists = select(exists(
        users.filter(email.eq(user_email)).filter(active.eq(true)),
    ))
    .get_result(conn)?;

    Ok(exists)
}
