use diesel::{BelongingToDsl, QueryDsl, RunQueryDsl, SelectableHelper, SqliteConnection};
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
