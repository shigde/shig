use crate::db::error::DbResult;
use crate::db::schema::verification_tokens::{token, verified};

use diesel::prelude::*;
use diesel::{RunQueryDsl, SelectableHelper, SqliteConnection};
use crate::db::verification_tokens::VerificationToken;

#[allow(dead_code)]
pub fn verify_verification_token(
    conn: &mut SqliteConnection,
    verify_token: &str,
) -> DbResult<VerificationToken> {
    use crate::db::schema::verification_tokens::dsl::verification_tokens;
    let verified_token = diesel::update(verification_tokens)
        .filter(token.eq(verify_token))
        .set(verified.eq(true))
        .returning(VerificationToken::as_returning())
        .get_result::<VerificationToken>(conn)
        .map_err(|e| -> String { format!("update verified token: {}", e) })?;

    Ok(verified_token)
}
