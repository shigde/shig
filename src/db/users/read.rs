use diesel::{select, BelongingToDsl, EqAll, QueryDsl, RunQueryDsl, SelectableHelper, SqliteConnection};
use diesel::dsl::exists;
use crate::db::actors::read::find_actor_by_actor_iri;
use crate::db::error::DbResult;
use crate::db::users::User;

pub fn find_user_by_actor_iri(conn: &mut SqliteConnection, iri: &str) -> DbResult<User> {
    let actor = find_actor_by_actor_iri(conn, iri)?;

    let user = User::belonging_to(&actor)
        .select(User::as_select())
        .first(conn)?;

    Ok(user)
}

pub fn exists_user_by_email(conn: &mut SqliteConnection, user_email: &str) -> DbResult<bool> {
    use crate::db::schema::users::dsl::*;
    let exists = select(exists(users.filter(email.eq_all(user_email)))).get_result(conn)?;
    Ok(exists)
}
