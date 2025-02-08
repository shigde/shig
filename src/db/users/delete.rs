use crate::db::actors::delete::delete_actor_by_id;
use crate::db::channels::delete::delete_channel_by_id;
use crate::db::channels::read::find_channel_by_user_id;
use crate::db::error::DbResult;
use crate::db::users::read::{find_user_by_email, find_user_by_id};
use crate::db::users::User;
use crate::db::verification_tokens::delete::delete_verification_token_by_user_id;
use diesel::prelude::*;
use diesel::{PgConnection, QueryDsl, RunQueryDsl};

pub fn delete_user_by_email(conn: &mut PgConnection, needl_email: &str) -> DbResult<()> {
    let user = find_user_by_email(conn, String::from(needl_email))?;
    delete_user_with_dependencies(conn, &user)?;

    Ok(())
}

pub fn delete_user_by_id(conn: &mut PgConnection, needle_id: i32) -> DbResult<()> {
    let user = find_user_by_id(conn, needle_id)?;
    delete_user_with_dependencies(conn, &user)?;

    Ok(())
}

fn delete_user_with_dependencies(conn: &mut PgConnection, user: &User) -> DbResult<()> {
    use crate::db::schema::users::active;
    use crate::db::schema::users::dsl::users;
    use crate::db::schema::users::id;

    let channel = find_channel_by_user_id(conn, user.id)?;
    delete_actor_by_id(conn, channel.actor_id)?;
    delete_actor_by_id(conn, user.actor_id)?;
    delete_channel_by_id(conn, channel.id)?;
    delete_verification_token_by_user_id(conn, user.id)?;
    diesel::delete(users.filter(id.eq(user.id)).filter(active.eq(false))).execute(conn)?;

    Ok(())
}
