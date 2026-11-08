use super::DeviceGrant;
use crate::db::{error::DbResult, schema::device_grants};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

pub fn redeem_invite(
    conn: &mut PgConnection,
    uuid: Uuid,
    invite_hash: &str,
    session_hash: &str,
    now: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> DbResult<Option<DeviceGrant>> {
    let invite = device_grants::table
        .find(uuid)
        .filter(device_grants::invite_hash.eq(invite_hash))
        .filter(device_grants::expires_at.gt(now));
    Ok(diesel::update(invite)
        .set((
            device_grants::invite_hash.eq(None::<&str>),
            device_grants::session_hash.eq(Some(session_hash)),
            device_grants::expires_at.eq(expires_at),
        ))
        .returning(DeviceGrant::as_returning())
        .get_result(conn)
        .optional()?)
}
