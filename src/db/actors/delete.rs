use crate::db::error::DbResult;
use diesel::prelude::*;

pub fn delete_actor_by_id(conn: &mut SqliteConnection, actor_id: i32) -> DbResult<()> {
    use crate::db::schema::actors::dsl::actors;
    use crate::db::schema::actors::id;

    diesel::delete(actors.filter(id.eq(actor_id))).execute(conn)?;
    Ok(())
}
