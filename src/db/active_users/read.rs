use crate::db::active_users::ActiveUser;
use crate::db::error::DbResult;
use diesel::prelude::*;
use diesel::{PgConnection, QueryDsl};

pub fn find_active_user_by_uuid(
    conn: &mut PgConnection,
    needle_uuid: &str,
) -> DbResult<ActiveUser> {
    use crate::db::schema_views::active_users::dsl::active_users;
    use crate::db::schema_views::active_users::user_uuid;

    let user = active_users
        .filter(user_uuid.eq(needle_uuid))
        .select(ActiveUser::as_select())
        .first(conn)?;

    Ok(user)
}
