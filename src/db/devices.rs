pub mod create;
pub mod delete;
pub mod read;
pub mod update;

use crate::db::{schema::devices, users::User};
use crate::models::error::ApiError;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use uuid::Uuid;

/// Persistent identity for an owner's camera. Lobby access is granted separately.
#[derive(Queryable, Identifiable, Selectable, Associations, Debug, PartialEq)]
#[diesel(table_name = devices)]
#[diesel(primary_key(uuid))]
#[diesel(belongs_to(User, foreign_key = owner_user_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Device {
    pub uuid: Uuid,
    pub owner_user_id: i32,
    pub slot: i16,
    pub name: String,
    pub active: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

fn device_name(name: &str) -> Result<&str, ApiError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 255 {
        return Err(ApiError::BadRequest {
            error_message: "Enter a device name of up to 255 characters".into(),
        });
    }
    Ok(name)
}
