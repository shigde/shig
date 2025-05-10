use chrono::{NaiveDateTime, Utc};
use diesel::{Insertable, PgConnection, RunQueryDsl, SelectableHelper};

use crate::db::error::DbResult;
use crate::db::stream_thumbnails::StreamThumbnail;

#[derive(Insertable, Debug)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = crate::db::schema::stream_thumbnails)]
pub struct NewStreamThumbnail<'a> {
    pub filename: &'a str,
    pub height: i32,
    pub width: i32,
    pub file_url: &'a str,
    pub on_disk: bool,
    pub stream_id: i32,
    pub created_at: NaiveDateTime,
}

pub fn insert_new_stream_thumbnail(
    conn: &mut PgConnection,
    filename: &str,
    height: i32,
    width: i32,
    file_url: &str,
    on_disk: bool,
    stream_id: i32,
) -> DbResult<StreamThumbnail> {
    let new_image = NewStreamThumbnail {
        filename,
        height,
        width,
        file_url,
        on_disk,
        stream_id,
        created_at: Utc::now().naive_utc(),
    };

    use crate::db::schema::stream_thumbnails;
    let image = diesel::insert_into(stream_thumbnails::table)
        .values(&new_image)
        .returning(StreamThumbnail::as_returning())
        .get_result::<StreamThumbnail>(conn)?;

    Ok(image)
}
