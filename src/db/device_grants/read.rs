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
