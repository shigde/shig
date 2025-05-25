use crate::db::error::DbResult;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{PgConnection, QueryDsl, RunQueryDsl};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, AsChangeset, Clone)]
#[diesel(table_name = crate::db::schema::streams)]
pub struct StreamUpdate<'a> {
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub support: Option<&'a str>,
    pub date: NaiveDateTime,
    pub start_time: Option<NaiveDateTime>,
    pub end_time: Option<NaiveDateTime>,
    pub licence: i32,
    pub is_repeating: bool,
    pub repeat: Option<i32>,
    pub is_public: bool,
    pub is_live: bool,
}

pub fn update_stream(
    conn: &mut PgConnection,
    stream_id: i32,
    title: &str,
    description: Option<&str>,
    support: Option<&str>,
    date: NaiveDateTime,
    start_time: Option<NaiveDateTime>,
    end_time: Option<NaiveDateTime>,
    licence: i32,
    is_repeating: bool,
    repeat: Option<i32>,
    is_public: bool,
    is_live: bool,
) -> DbResult<()> {
    let stream = StreamUpdate {
        title,
        description: description.to_owned(),
        support: support.to_owned(),
        date: date.to_owned(),
        start_time: start_time.to_owned(),
        end_time: end_time.to_owned(),
        licence,
        is_repeating,
        repeat: repeat.to_owned(),
        is_public,
        is_live,
    };
    use crate::db::schema::streams::dsl::id;
    use crate::db::schema::streams::dsl::streams;
    diesel::update(streams.filter(id.eq(stream_id)))
        .set::<StreamUpdate>(stream)
        .execute(conn)?;
    Ok(())
}
