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

pub fn find_if_exist_active_user_by_uuid(
    conn: &mut PgConnection,
    needle_uuid: &str,
) -> DbResult<Option<ActiveUser>> {
    use crate::db::schema_views::active_users::dsl::active_users;
    use crate::db::schema_views::active_users::user_uuid;

    let user = active_users
        .filter(user_uuid.eq(needle_uuid))
        .select(ActiveUser::as_select())
        .first(conn)
        .optional()?;

    Ok(user)
}

pub fn search_active_users_by_name(conn: &mut PgConnection, query: &str) -> DbResult<Vec<ActiveUser>> {
    if query.len() < 2 {
        return Ok(vec![]);
    }

    use crate::db::schema_views::active_users::dsl::active_users;
    use crate::db::schema_views::active_users::name;

    let escaped = escape_like(query.trim());
    let pattern = format!("%{}%", escaped);

    let list = active_users
        .filter(name.ilike(pattern).escape('\\'))
        .order(name.asc())
        .limit(20)
        .load::<ActiveUser>(conn)?;
    Ok(list)
}

fn escape_like(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}
