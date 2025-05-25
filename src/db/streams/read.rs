use crate::db::error::DbResult;
use crate::db::streams::Stream;
use diesel::prelude::*;
use diesel::{PgConnection, QueryDsl, RunQueryDsl, SelectableHelper};
use serde::Serialize;
use crate::db::stream_meta_data::StreamMetaData;

#[derive(Serialize)]
#[allow(dead_code)] struct FullyLoadedStream {
    #[serde(flatten)]
    stream: Stream,
    meta_data: Vec<StreamMetaData>,
}

pub fn find_stream_by_uuid(conn: &mut PgConnection, stream_uuid: String) -> DbResult<Stream> {
    use crate::db::schema::streams::dsl::streams;
    use crate::db::schema::streams::uuid;
    let stream_dao = streams
        .filter(uuid.eq(stream_uuid))
        .select(Stream::as_select())
        .first(conn)?;

    Ok(stream_dao)
}
