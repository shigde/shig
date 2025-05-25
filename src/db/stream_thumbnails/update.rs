use crate::db::error::DbResult;
use diesel::prelude::*;
use diesel::{Insertable, PgConnection, RunQueryDsl};

#[derive(Insertable, AsChangeset, Debug)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = crate::db::schema::stream_thumbnails)]
pub struct StreamThumbnailUpdate<'a> {
    pub filename: &'a str,
    pub height: i32,
    pub width: i32,
    pub file_url: &'a str,
    pub on_disk: bool,
}

pub fn update_stream_thumbnail(
    conn: &mut PgConnection,
    thumbnail_id: i32,
    thumbnail: StreamThumbnailUpdate,
) -> DbResult<()> {
    use crate::db::schema::stream_thumbnails::dsl::id;
    use crate::db::schema::stream_thumbnails::dsl::stream_thumbnails;
    diesel::update(stream_thumbnails.filter(id.eq(thumbnail_id)))
        .set::<StreamThumbnailUpdate>(thumbnail)
        .execute(conn)?;
    Ok(())
}
