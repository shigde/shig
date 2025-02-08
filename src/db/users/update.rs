use crate::db::error::DbResult;
use crate::db::users::User;
use crate::db::verification_tokens::update::verify_verification_token;
use diesel::{Connection, EqAll, PgConnection, QueryDsl, RunQueryDsl, SelectableHelper};

pub fn activate_user_by_verification_token(
    conn: &mut PgConnection,
    token: &str,
) -> DbResult<User> {
    conn.transaction(move |conn| {
        let verification_token = verify_verification_token(conn, token)?;

        use crate::db::schema::users::active;
        use crate::db::schema::users::dsl::users;
        let updated_user = diesel::update(users.find(verification_token.user_id))
            .set(active.eq_all(true))
            .returning(User::as_returning())
            .get_result(conn)?;
        Ok(updated_user)
    })
}
