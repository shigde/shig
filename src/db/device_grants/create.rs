use crate::db::{error::DbResult, schema::device_grants};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::upsert::excluded;
use uuid::Uuid;

pub fn store_invite(
    conn: &mut PgConnection,
    uuid: Uuid,
    stream_id: i32,
    invite_hash: &str,
    expires_at: DateTime<Utc>,
) -> DbResult<()> {
    diesel::insert_into(device_grants::table)
        .values((
            device_grants::device_uuid.eq(uuid),
            device_grants::stream_id.eq(stream_id),
            device_grants::invite_hash.eq(Some(invite_hash)),
            device_grants::session_hash.eq(None::<&str>),
            device_grants::expires_at.eq(expires_at),
        ))
        .on_conflict(device_grants::device_uuid)
        .do_update()
        .set((
            device_grants::stream_id.eq(excluded(device_grants::stream_id)),
            device_grants::invite_hash.eq(excluded(device_grants::invite_hash)),
            device_grants::session_hash.eq(excluded(device_grants::session_hash)),
            device_grants::expires_at.eq(excluded(device_grants::expires_at)),
        ))
        .execute(conn)?;
    Ok(())
}
