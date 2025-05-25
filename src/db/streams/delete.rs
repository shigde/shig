use crate::db::error::DbResult;
use diesel::prelude::*;

pub fn delete_stream_by_id(conn: &mut PgConnection, stream_id: i32) -> DbResult<()> {
    use crate::db::schema::streams::dsl::streams;
    use crate::db::schema::streams::id;

    diesel::delete(streams.filter(id.eq(stream_id))).execute(conn)?;
    Ok(())
}
