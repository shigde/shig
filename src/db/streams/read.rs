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

pub fn find_stream_by_uuid(
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
