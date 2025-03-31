use crate::db::error::DbResult;
use crate::db::sessions::Principal;
use diesel::prelude::*;
use diesel::{PgConnection, QueryDsl};

pub fn find_principal_by_uuid(conn: &mut PgConnection, needle_uuid: String) -> DbResult<Principal> {
    use crate::db::schema_views::session_principals_view::dsl::session_principals_view;
    use crate::db::schema_views::session_principals_view::user_uuid;

    let principal = session_principals_view
        .filter(user_uuid.eq(needle_uuid))
        .select(Principal::as_select())
        .first(conn)?;

    Ok(principal)
}
