use crate::db::error::DbResult;
use crate::db::stream_thumbnails::StreamThumbnail;
use diesel::{PgConnection, QueryDsl, RunQueryDsl, SelectableHelper};
use diesel::prelude::*;

#[allow(dead_code)]
pub fn find_thumbnail_by_stream_id(
    conn: &mut PgConnection,
    stream: i32,
) -> DbResult<StreamThumbnail> {
    use crate::db::schema::stream_thumbnails;
    use crate::db::schema::stream_thumbnails::dsl::*;
    let image = stream_thumbnails::table
        .filter(stream_id.eq(stream))
        .select(StreamThumbnail::as_select())
        .get_result(conn)?;

    Ok(image)
}
