use crate::db::error::DbResult;
use diesel::prelude::*;


pub fn delete_stream_participant_by_user_and_stream_uuid(
    conn: &mut PgConnection,
    stream_uuid: &str,
    user_uuid: &str,
) -> DbResult<usize> {
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

    let result = diesel::delete(
        sp::stream_participants
            .filter(sp::user_id.eq_any(user_id_subquery))
            .filter(sp::stream_id.eq_any(stream_id_subquery)),
    ).execute(conn)?;

    Ok(result)
}
