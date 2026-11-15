use super::{device_name, Device};
use crate::db::schema::{devices, users};
use crate::models::error::ApiError;
use diesel::prelude::*;
use uuid::Uuid;

pub fn create(conn: &mut PgConnection, owner: i32, name: &str) -> Result<Device, ApiError> {
    let name = device_name(name)?;
    conn.transaction(|conn| {
        users::table
            .find(owner)
            .select(users::id)
            .for_update()
            .first::<i32>(conn)?;
        let occupied = devices::table
            .filter(devices::owner_user_id.eq(owner))
            .select(devices::slot)
            .load::<i16>(conn)?;
        let slot = (1..=3)
            .find(|slot| !occupied.contains(slot))
            .ok_or_else(|| ApiError::Conflict {
                error_message: "All three device slots are in use".into(),
            })?;
        Ok(diesel::insert_into(devices::table)
            .values((
                devices::uuid.eq(Uuid::new_v4()),
                devices::owner_user_id.eq(owner),
                devices::slot.eq(slot),
                devices::name.eq(name),
            ))
            .returning(Device::as_returning())
            .get_result(conn)?)
    })
}
