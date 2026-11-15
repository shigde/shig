use crate::db::schema::devices;
use crate::models::error::ApiError;
use diesel::prelude::*;
use uuid::Uuid;

pub fn delete(conn: &mut PgConnection, owner: i32, uuid: Uuid) -> Result<(), ApiError> {
    let deleted = diesel::delete(
        devices::table
            .find(uuid)
            .filter(devices::owner_user_id.eq(owner)),
    )
    .execute(conn)?;
    if deleted == 0 {
        return Err(ApiError::NotFound {
            error_message: "Device not found".into(),
        });
    }
    Ok(())
}
