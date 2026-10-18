use crate::db::error::DbResult;
use diesel::prelude::*;

pub fn delete_stream_by_id(conn: &mut PgConnection, stream_id: i32) -> DbResult<bool> {
    use crate::db::schema::streams::dsl::streams;
    use crate::db::schema::streams::id;
    use crate::db::schema::streams::is_live;

    let deleted =
        diesel::delete(streams.filter(id.eq(stream_id)).filter(is_live.eq(false))).execute(conn)?;
    Ok(deleted != 0)
}
