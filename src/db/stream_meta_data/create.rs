use chrono::{NaiveDateTime};
use diesel::{Insertable, PgConnection, RunQueryDsl, SelectableHelper};
use crate::db::error::DbResult;
use crate::db::stream_meta_data::StreamMetaData;

#[derive(Insertable, Debug)]
#[diesel(belongs_to(Stream))]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = crate::db::schema::stream_meta_data)]
pub struct NewStreamMetaData<'a> {
    pub stream_id: i32,
    pub is_shig: bool,
    pub stream_key: &'a str,
    pub url: &'a str,
    pub protocol: i32,
    pub permanent_live: bool,
    pub save_replay: bool,
    pub latency_mode: i32,
    pub created_at: NaiveDateTime,
}

#[allow(dead_code)]
pub fn insert_new_stream_meta_data(
    conn: &mut PgConnection,
    new_stream_meta_data: NewStreamMetaData,
) -> DbResult<StreamMetaData> {

    use crate::db::schema::stream_meta_data;
    let image = diesel::insert_into(stream_meta_data::table)
        .values(&new_stream_meta_data)
        .returning(StreamMetaData::as_returning())
        .get_result::<StreamMetaData>(conn)?;

    Ok(image)
}
