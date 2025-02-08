use crate::db::error::DbResult;
use crate::db::schema::verification_tokens::{token, verified, created_at};
use chrono::Utc;
use std::time::Duration;

use crate::db::verification_tokens::VerificationToken;
use diesel::prelude::*;
use diesel::{RunQueryDsl, SelectableHelper, PgConnection};

#[allow(dead_code)]
pub fn verify_verification_token(
    conn: &mut PgConnection,
    verify_token: &str,
) -> DbResult<VerificationToken> {
    use crate::db::schema::verification_tokens::dsl::verification_tokens;

    // 30 minutes
    let ten_minutes = Utc::now() - Duration::from_secs(60 * 30);

    let verified_token = diesel::update(verification_tokens)
        .filter(token.eq(verify_token))
        .filter(verified.eq(false))
        .filter(created_at.gt(ten_minutes.naive_utc()))
        .set(verified.eq(true))
        .returning(VerificationToken::as_returning())
        .get_result::<VerificationToken>(conn)
        .map_err(|e| -> String { format!("update verified token: {}", e) })?;

    Ok(verified_token)
}
