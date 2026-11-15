use crate::db::error::DbErrorKind::Internal;
use crate::db::error::{DbError, DbResult};
use crate::db::stream_meta_data::StreamMetaData;
use crate::db::stream_thumbnails::StreamThumbnail;
use crate::db::streams::Stream;
use diesel::prelude::*;
use diesel::{PgConnection, QueryDsl, RunQueryDsl, SelectableHelper};
use serde::Serialize;

#[derive(Serialize)]
pub struct FullyLoadedStream {
    #[serde(flatten)]
    pub stream: Stream,
    pub meta_data: StreamMetaData,
    pub thumbnail: Option<StreamThumbnail>,
}

pub fn find_full_stream_by_uuid(
    conn: &mut PgConnection,
    stream_uuid: String,
) -> DbResult<FullyLoadedStream> {
    use crate::db::schema::streams::dsl::streams;
    use crate::db::schema::streams::uuid;
    let stream_dao = streams
        .filter(uuid.eq(stream_uuid.to_owned()))
        .select(Stream::as_select())
        .first(conn)?;

    let image_list: Vec<StreamThumbnail> = StreamThumbnail::belonging_to(&stream_dao)
        .select(StreamThumbnail::as_select())
        .load(conn)?;

    let meta_data_list: Vec<StreamMetaData> = StreamMetaData::belonging_to(&stream_dao)
        .select(StreamMetaData::as_select())
        .load(conn)?;

    match meta_data_list.first() {
        None => Err(DbError::new(
            format!("No meta data found for stream: {}", stream_uuid.as_str()),
            Internal,
        )),
        Some(meta) => Ok(FullyLoadedStream {
            stream: stream_dao,
            meta_data: meta.clone(),
            thumbnail: image_list.first().cloned(),
        }),
    }
}

#[allow(dead_code)]
pub fn find_stream_by_uuid(conn: &mut PgConnection, stream_uuid: String) -> DbResult<Stream> {
    use crate::db::schema::streams::dsl::streams;
    use crate::db::schema::streams::uuid;
    let stream_dao = streams
        .filter(uuid.eq(stream_uuid.to_owned()))
        .select(Stream::as_select())
        .first(conn)?;
    Ok(stream_dao)
}

pub fn find_if_exists_stream_by_uuid_for_owner(
    conn: &mut PgConnection,
    stream_uuid: &str,
    owner_user_id: i32,
) -> DbResult<Option<Stream>> {
    use crate::db::schema::streams::dsl::*;

    let stream_dao = streams
        .filter(uuid.eq(stream_uuid))
        .filter(user_id.eq(owner_user_id))
        .select(Stream::as_select())
        .first::<Stream>(conn)
        .optional()?;
    Ok(stream_dao)
}

pub fn lock_owned_stream_id(
    conn: &mut PgConnection,
    stream_uuid: &str,
    owner: i32,
) -> DbResult<i32> {
    use crate::db::schema::streams;

    Ok(streams::table
        .filter(streams::uuid.eq(stream_uuid))
        .filter(streams::user_id.eq(owner))
        .select(streams::id)
        .for_update()
        .first(conn)?)
}

pub fn find_stream_uuid_by_id(conn: &mut PgConnection, id: i32) -> QueryResult<String> {
    use crate::db::schema::streams;

    streams::table.find(id).select(streams::uuid).first(conn)
}

pub fn lock_stream(conn: &mut PgConnection, id: i32) -> DbResult<()> {
    use crate::db::schema::streams;

    streams::table.find(id)
        .select(streams::id)
        .for_update()
        .first::<i32>(conn)?;
    Ok(())
}
