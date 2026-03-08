use diesel::PgConnection;
use crate::db::error::DbResult;
use crate::db::stream_participants::StreamParticipant;

pub fn read_stream_participant_by_user_and_stream_uuid(
    conn: &mut PgConnection,
    stream_uuid: &str,
    user_uuid: &str,
) -> DbResult<Option<StreamParticipant>> {
    use diesel::prelude::*;
    use crate::db::schema::{
        stream_participants::dsl as sp,
        users::dsl as u,
        streams::dsl as s,
    };

    // Subquery: find user_id
    let user_id_subquery = u::users
        .filter(u::user_uuid.eq(user_uuid))
        .select(u::id);

    // Subquery: find stream_id
    let stream_id_subquery = s::streams
        .filter(s::uuid.eq(stream_uuid))
        .select(s::id);

    let participant = sp::stream_participants
        .filter(sp::user_id.eq_any(user_id_subquery))
        .filter(sp::stream_id.eq_any(stream_id_subquery))
        .first::<StreamParticipant>(conn)
        .optional()?;

    Ok(participant)
}