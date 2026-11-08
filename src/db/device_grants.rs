pub mod create;
pub mod delete;
pub mod read;
pub mod update;

use crate::db::{devices::Device, schema::device_grants, streams::Stream};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

#[derive(Queryable, Identifiable, Selectable, Associations)]
#[diesel(table_name = device_grants)]
#[diesel(primary_key(device_uuid))]
#[diesel(belongs_to(Device, foreign_key = device_uuid))]
#[diesel(belongs_to(Stream))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DeviceGrant {
    pub device_uuid: Uuid,
    pub stream_id: i32,
    pub invite_hash: Option<String>,
    pub session_hash: Option<String>,
    pub expires_at: DateTime<Utc>,
}
