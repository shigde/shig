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
    filename: &str,
    height: i32,
    width: i32,
    file_url: &str,
    on_disk: bool,
) -> DbResult<()> {
    let update_image = StreamThumbnailUpdate {
        filename,
        height,
        width,
        file_url,
        on_disk,
    };

    use crate::db::schema::stream_thumbnails::dsl::id;
    use crate::db::schema::stream_thumbnails::dsl::stream_thumbnails;
    diesel::update(stream_thumbnails.filter(id.eq(thumbnail_id)))
        .set::<StreamThumbnailUpdate>(update_image)
        .execute(conn)?;
    Ok(())
}
