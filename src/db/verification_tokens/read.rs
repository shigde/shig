use crate::db::error::DbResult;
use crate::db::verification_tokens::{
    FORGOTTEN_PASSWORD_VERIFICATION_TOKEN, SIGN_UP_VERIFICATION_TOKEN,
};
use diesel::prelude::*;
use diesel::dsl::exists;
use diesel::{select, EqAll, ExpressionMethods, RunQueryDsl, SqliteConnection};

#[allow(dead_code)]
pub fn exists_sing_up_verification_tokens(
    conn: &mut SqliteConnection,
    verify_token: &str,
) -> DbResult<bool> {

    use crate::db::schema::verification_tokens;

    let exists = select(exists(
        verification_tokens::dsl::verification_tokens
            .filter(verification_tokens::kind.eq_all(SIGN_UP_VERIFICATION_TOKEN))
            .filter(verification_tokens::verified.eq(false))
            .filter(verification_tokens::token.eq_all(verify_token)),
    ))
    .get_result(conn)?;
    Ok(exists)
}

#[allow(dead_code)]
pub fn exists_forgotten_pass_verification_tokens(
    conn: &mut SqliteConnection,
    verify_token: &str,
) -> DbResult<bool> {

    use crate::db::schema::verification_tokens;

    let exists = select(diesel::dsl::exists(
        verification_tokens::dsl::verification_tokens
            .filter(verification_tokens::kind.eq_all(FORGOTTEN_PASSWORD_VERIFICATION_TOKEN))
            .filter(verification_tokens::verified.eq(false))
            .filter(verification_tokens::token.eq_all(verify_token)),
    ))
    .get_result(conn)?;
    Ok(exists)
}
