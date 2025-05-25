use crate::db::error::DbResult;
use diesel::prelude::*;

use diesel::{AsChangeset, Insertable, PgConnection, RunQueryDsl};

#[derive(Insertable, AsChangeset, Debug)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = crate::db::schema::stream_meta_data)]
pub struct StreamMetaDataUpdate<'a> {
    pub stream_id: i32,
    pub is_shig: bool,
    pub stream_key: &'a str,
    pub url: &'a str,
    pub protocol: i32,
    pub permanent_live: bool,
    pub save_replay: bool,
    pub latency_mode: i32,
}

pub fn update_stream_meta_data(
    conn: &mut PgConnection,
    stream_meta_data_id: i32,
    update_dao: StreamMetaDataUpdate,
) -> DbResult<()> {
    use crate::db::schema::stream_meta_data::dsl::id;
    use crate::db::schema::stream_meta_data::dsl::stream_meta_data;

    diesel::update(stream_meta_data.filter(id.eq(stream_meta_data_id)))
        .set::<StreamMetaDataUpdate>(update_dao)
        .execute(conn)?;
    Ok(())
}
