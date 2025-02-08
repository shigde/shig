use crate::db::error::DbResult;
use crate::db::verification_tokens::{
    VerificationToken, FORGOTTEN_PASSWORD_VERIFICATION_TOKEN, SIGN_UP_VERIFICATION_TOKEN,
};
use diesel::dsl::exists;
use diesel::prelude::*;
use diesel::{select, ExpressionMethods, RunQueryDsl, PgConnection};

#[allow(dead_code)]
pub fn exists_sing_up_verification_tokens(
    conn: &mut PgConnection,
    verify_token: &str,
) -> DbResult<bool> {
    use crate::db::schema::verification_tokens;

    let exists = select(exists(
        verification_tokens::dsl::verification_tokens
            .filter(verification_tokens::kind.eq(SIGN_UP_VERIFICATION_TOKEN))
            .filter(verification_tokens::verified.eq(false))
            .filter(verification_tokens::token.eq(verify_token)),
    ))
    .get_result(conn)?;
    Ok(exists)
}

#[allow(dead_code)]
pub fn exists_forgotten_pass_verification_tokens(
    conn: &mut PgConnection,
    verify_token: &str,
) -> DbResult<bool> {
    use crate::db::schema::verification_tokens;

    let exists = select(diesel::dsl::exists(
        verification_tokens::dsl::verification_tokens
            .filter(verification_tokens::kind.eq(FORGOTTEN_PASSWORD_VERIFICATION_TOKEN))
            .filter(verification_tokens::verified.eq(false))
            .filter(verification_tokens::token.eq(verify_token)),
    ))
    .get_result(conn)?;
    Ok(exists)
}

pub fn find_sing_up_verification_token(
    conn: &mut PgConnection,
    user_id: i32,
) -> DbResult<VerificationToken> {
    use crate::db::schema::verification_tokens;

    let token = verification_tokens::table
        .filter(verification_tokens::kind.eq(SIGN_UP_VERIFICATION_TOKEN))
        .filter(verification_tokens::verified.eq(false))
        .filter(verification_tokens::user_id.eq(user_id))
        .order_by(verification_tokens::created_at.desc())
        .select(VerificationToken::as_select())
        .first(conn)?;

    Ok(token)
}
