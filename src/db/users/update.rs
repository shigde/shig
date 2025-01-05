use diesel::{EqAll, QueryDsl, RunQueryDsl, SelectableHelper, SqliteConnection};
use crate::db::error::DbResult;
use crate::db::users::User;

#[allow(dead_code)]
pub fn activate_user(conn: &mut SqliteConnection, user_id: i32) -> DbResult<User> {

    use crate::db::schema::users::dsl::users;
    use crate::db::schema::users::active;
    let updated_user = diesel::update(users.find(user_id))
        .set(active.eq_all(true))
        .returning(User::as_returning())
        .get_result(conn)?;

    Ok(updated_user)
}
