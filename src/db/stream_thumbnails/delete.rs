use crate::db::error::DbResult;
use diesel::prelude::*;

#[allow(dead_code)]
pub fn delete_stream_thumbnail_by_id(conn: &mut PgConnection, thumbnail_id: i32) -> DbResult<()> {
    use crate::db::schema::stream_thumbnails::dsl::stream_thumbnails;
    use crate::db::schema::stream_thumbnails::id;

    diesel::delete(stream_thumbnails.filter(id.eq(thumbnail_id))).execute(conn)?;
    Ok(())
}
