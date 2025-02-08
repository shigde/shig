use diesel::{RunQueryDsl, PgConnection};
use crate::db::error::DbResult;
use diesel::prelude::*;

pub fn delete_verification_token_by_user_id(conn: &mut PgConnection, find_user_id: i32) -> DbResult<()> {
    use crate::db::schema::verification_tokens::dsl::verification_tokens;
    use crate::db::schema::verification_tokens::user_id;

    diesel::delete(verification_tokens.filter(user_id.eq(find_user_id))).execute(conn)?;
    Ok(())
}
