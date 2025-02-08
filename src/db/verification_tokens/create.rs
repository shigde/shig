use chrono::{NaiveDateTime, Utc};
use diesel::{Insertable, RunQueryDsl, SelectableHelper, PgConnection};
use uuid::Uuid;
use crate::db::error::DbResult;
use crate::db::verification_tokens::VerificationToken;

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::db::schema::verification_tokens)]
pub struct NewVerificationToken<'a> {
    pub user_id: i32,
    pub kind: &'a str,
    pub token: &'a str,
    pub verified: bool,
    pub created_at: NaiveDateTime,
}

pub fn insert_new_verification_token(
    conn: &mut PgConnection,
    user_id: i32,
    kind: &str,
) -> DbResult<VerificationToken> {
    let token = Uuid::new_v4().to_string();
    let new_verify_token = NewVerificationToken {
        user_id,
        kind,
        token: token.as_str(),
        verified: false,
        created_at: Utc::now().naive_utc(),
    };

    use crate::db::schema::verification_tokens::dsl::verification_tokens;
    let verify_token = diesel::insert_into(verification_tokens)
        .values(&new_verify_token)
        .returning(VerificationToken::as_returning())
        .get_result::<VerificationToken>(conn)
        .map_err(|e| -> String { format!("create new verify token: {}", e) })?;

    Ok(verify_token)
}
