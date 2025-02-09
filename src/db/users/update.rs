use crate::db::error::{DbError, DbErrorKind, DbResult};
use crate::db::users::read::find_user_by_id;
use crate::db::users::User;
use crate::db::verification_tokens::update::verify_verification_token;
use crate::db::verification_tokens::{
    FORGOTTEN_PASSWORD_VERIFICATION_TOKEN, SIGN_UP_VERIFICATION_TOKEN,
};
use bcrypt::{hash, DEFAULT_COST};
// use diesel::prelude::*;
use diesel::{
    Connection, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl, SelectableHelper,
};

pub fn activate_user_by_verification_token(conn: &mut PgConnection, token: &str) -> DbResult<User> {
    conn.transaction(move |conn| {
        let verification_token =
            verify_verification_token(conn, token, SIGN_UP_VERIFICATION_TOKEN)?;

        use crate::db::schema::users::dsl::active;
        use crate::db::schema::users::dsl::users;
        let updated_user = diesel::update(users.find(verification_token.user_id))
            .set(active.eq(true))
            .returning(User::as_returning())
            .get_result(conn)?;

        Ok(updated_user)
    })
}

pub fn update_password_by_token(
    conn: &mut PgConnection,
    token: &str,
    new_password: &str,
) -> DbResult<User> {
    conn.transaction(move |conn| {
        let verification_token =
            verify_verification_token(conn, token, FORGOTTEN_PASSWORD_VERIFICATION_TOKEN)?;
        let hashed_pass: String = hash(new_password, DEFAULT_COST)
            .map_err(|e| -> String { format!("failed to hash pass word: {}", e) })?;

        use crate::db::schema::users::dsl::password;
        use crate::db::schema::users::dsl::users;
        let updated_user = diesel::update(users.find(verification_token.user_id))
            .set(password.eq(hashed_pass))
            .returning(User::as_returning())
            .get_result(conn)?;

        Ok(updated_user)
    })
}

pub fn update_password_by_id(
    conn: &mut PgConnection,
    user_id: i32,
    new_password: &str,
    old_password: &str,
) -> DbResult<User> {
    conn.transaction(move |conn| {
        let hashed_pass: String = hash(new_password, DEFAULT_COST)
            .map_err(|e| -> String { format!("failed to hash pass word: {}", e) })?;

        let user = find_user_by_id(conn, user_id)?;

        if !user.verify(old_password.to_string()) {
            return Err(DbError::new(
                String::from("wrong passwort"),
                DbErrorKind::NotFound,
            ));
        }

        use crate::db::schema::users::dsl::password;
        use crate::db::schema::users::dsl::users;
        let updated_user = diesel::update(users.find(user_id))
            .set(password.eq(hashed_pass))
            .returning(User::as_returning())
            .get_result(conn)?;

        Ok(updated_user)
    })
}
