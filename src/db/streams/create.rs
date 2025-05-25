use crate::db::error::DbResult;
use chrono::{NaiveDateTime, Utc};
use diesel::{Insertable, PgConnection, RunQueryDsl, SelectableHelper};
use crate::db::streams::Stream;

#[derive(Insertable,Debug)]
#[diesel(table_name = crate::db::schema::streams)]
pub struct NewStream<'a> {
    pub uuid: &'a str,
    pub user_id: i32,
    pub channel_id: i32,
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
    pub created_at: NaiveDateTime,
}

pub fn insert_new_stream(
    conn: &mut PgConnection,
    uuid: &str,
    user_id: i32,
    channel_id: i32,
    title: &str,
    description: Option<&str>,
    support: Option<&str>,
    date: NaiveDateTime,
    licence: i32,
    is_repeating: bool,
    repeat: Option<i32>,
    is_public: bool,
) -> DbResult<Stream> {
    let new_stream = NewStream {
        uuid,
        user_id,
        channel_id,
        title,
        description: description.to_owned(),
        support: support.to_owned(),
        date: date.to_owned(),
        start_time: None,
        end_time: None,
        licence,
        is_repeating: is_repeating,
        repeat: repeat.to_owned(),
        is_public,
        is_live: false,
        created_at: Utc::now().naive_utc().clone(),
    };

    use crate::db::schema::streams::dsl::streams;
    let stream = diesel::insert_into(streams)
        .values(&new_stream)
        .returning(Stream::as_returning())
        .get_result::<Stream>(conn)
        .map_err(|e| -> String { format!("create stream: {}", e) })?;

    Ok(stream)
}
