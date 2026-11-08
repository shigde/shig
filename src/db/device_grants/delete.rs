use crate::db::{error::DbResult, schema::device_grants};
use diesel::prelude::*;
use uuid::Uuid;

pub fn delete_by_device(conn: &mut PgConnection, uuid: Uuid) -> DbResult<()> {
    diesel::delete(device_grants::table.find(uuid)).execute(conn)?;
    Ok(())
}

pub fn revoke_stream(conn: &mut PgConnection, stream_id: i32) -> DbResult<Vec<Uuid>> {
    Ok(diesel::delete(device_grants::table.filter(device_grants::stream_id.eq(stream_id)))
        .returning(device_grants::device_uuid)
        .get_results(conn)?)
}

pub fn delete_by_session(conn: &mut PgConnection, session_hash: &str) -> DbResult<bool> {
    Ok(diesel::delete(device_grants::table.filter(device_grants::session_hash.eq(session_hash)))
        .execute(conn)? != 0)
}
