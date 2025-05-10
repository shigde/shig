use crate::db::error::DbResult;
use diesel::prelude::*;

#[allow(dead_code)]
pub fn delete_actor_image_by_id(conn: &mut PgConnection, actor_id: i32) -> DbResult<()> {
    use crate::db::schema::actor_images::dsl::actor_images;
    use crate::db::schema::actor_images::id;

    diesel::delete(actor_images.filter(id.eq(actor_id))).execute(conn)?;
    Ok(())
}
