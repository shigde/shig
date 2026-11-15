use super::{device_name, read::lock_owned, Device};
use crate::db::{device_grants, schema::devices};
use crate::models::error::ApiError;
use diesel::prelude::*;
use uuid::Uuid;

pub fn update(
    conn: &mut PgConnection,
    owner: i32,
    uuid: Uuid,
    name: Option<&str>,
    active: Option<bool>,
) -> Result<Device, ApiError> {
    let name = name.map(device_name).transpose()?;
    conn.transaction(|conn| {
        let device = lock_owned(conn, owner, uuid)?;
        let active = active.unwrap_or(device.active);
        if !active {
            device_grants::delete::delete_by_device(conn, uuid)?;
        }
        Ok(diesel::update(devices::table.find(uuid))
            .set((
                devices::name.eq(name.unwrap_or(&device.name)),
                devices::active.eq(active),
            ))
            .returning(Device::as_returning())
            .get_result(conn)?)
    })
}
