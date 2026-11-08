use super::Device;
use crate::db::{error::DbResult, schema::devices};
use diesel::prelude::*;
use uuid::Uuid;

pub fn list(conn: &mut PgConnection, owner: i32) -> DbResult<Vec<Device>> {
    Ok(devices::table
        .filter(devices::owner_user_id.eq(owner))
        .order(devices::slot)
        .select(Device::as_select())
        .load(conn)?)
}

pub fn lock_owned(conn: &mut PgConnection, owner: i32, uuid: Uuid) -> DbResult<Device> {
    Ok(devices::table
        .find(uuid)
        .filter(devices::owner_user_id.eq(owner))
        .select(Device::as_select())
        .for_update()
        .first(conn)?)
}

pub fn lock_active_name(conn: &mut PgConnection, uuid: Uuid) -> DbResult<Option<String>> {
    Ok(devices::table
        .find(uuid)
        .filter(devices::active)
        .select(devices::name)
        .for_update()
        .first(conn)
        .optional()?)
}
