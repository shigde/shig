use super::DeviceGrant;
use crate::db::{error::DbResult, schema::device_grants};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

pub fn has_active_session(
    conn: &mut PgConnection,
    uuid: Uuid,
    now: DateTime<Utc>,
) -> DbResult<bool> {
    let active = device_grants::table
        .find(uuid)
        .filter(device_grants::session_hash.is_not_null())
        .filter(device_grants::expires_at.gt(now));
    Ok(diesel::select(diesel::dsl::exists(active)).get_result(conn)?)
}

pub fn find_invite(conn: &mut PgConnection, invite_hash: &str) -> DbResult<Option<DeviceGrant>> {
    Ok(device_grants::table
        .filter(device_grants::invite_hash.eq(invite_hash))
        .select(DeviceGrant::as_select())
        .first(conn)
        .optional()?)
}

pub fn lock_session(conn: &mut PgConnection, session_hash: &str) -> DbResult<Option<DeviceGrant>> {
    Ok(device_grants::table
        .filter(device_grants::session_hash.eq(session_hash))
        .filter(device_grants::expires_at.gt(Utc::now()))
        .select(DeviceGrant::as_select())
        .for_update()
        .first(conn)
        .optional()?)
}

pub fn is_active_for(
    conn: &mut PgConnection,
    session_hash: &str,
    device_id: Uuid,
    stream_id: i32,
    owner_id: i32,
) -> DbResult<bool> {
    use crate::db::schema::devices;

    let active = device_grants::table
        .inner_join(devices::table)
        .filter(device_grants::device_uuid.eq(device_id))
        .filter(device_grants::stream_id.eq(stream_id))
        .filter(device_grants::session_hash.eq(session_hash))
        .filter(device_grants::expires_at.gt(Utc::now()))
        .filter(devices::owner_user_id.eq(owner_id))
        .filter(devices::active);
    Ok(diesel::select(diesel::dsl::exists(active)).get_result(conn)?)
}
