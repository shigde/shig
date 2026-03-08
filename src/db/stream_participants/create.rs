use crate::db::error::DbResult;
use crate::db::stream_participants::StreamParticipant;
use chrono::{NaiveDateTime, Utc};
use diesel::{Insertable, PgConnection};

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::db::schema::stream_participants)]
pub struct NewStreamParticipant {
    pub user_id: i32,
    pub stream_id: i32,
    pub created_at: NaiveDateTime,
}

pub fn insert_new_stream_participant(
    conn: &mut PgConnection,
    stream_uuid: &str,
    user_uuid: &str,
) -> DbResult<StreamParticipant> {
    use crate::db::schema::stream_participants::dsl as sp;
    use crate::db::schema::{streams as s, users as u};
    use diesel::prelude::*;

    let user_id = u::table
        .filter(u::user_uuid.eq(user_uuid))
        .select(u::id)
        .first::<i32>(conn)
        .optional()?
        .ok_or(format!(
            " User not found to create StreamParticipant (user_uuid={}, stream_uuid={})",
            user_uuid, stream_uuid
        ))?;

    let stream_id = s::table
        .filter(s::uuid.eq(stream_uuid))
        .select(s::id)
        .first::<i32>(conn)
        .optional()?
        .ok_or(format!(
            " Stream not found to create StreamParticipant (user_uuid={}, stream_uuid={})",
            user_uuid, stream_uuid
        ))?;

    let participant = diesel::insert_into(sp::stream_participants)
        .values((
            sp::user_id.eq(user_id),
            sp::stream_id.eq(stream_id),
            sp::created_at.eq(Utc::now().naive_utc()),
        ))
        .returning(StreamParticipant::as_returning())
        .get_result::<StreamParticipant>(conn)
        .map_err(|e| -> String {
            format!(
                "create StreamParticipant (user_uuid={}, stream_uuid={}): {}",
                user_uuid, stream_uuid, e
            )
        })?;

    Ok(participant)
}
