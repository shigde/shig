pub mod grant;
pub mod publish;

use crate::db::devices::Device;
use serde::Serialize;

#[derive(Serialize)]
pub struct DeviceView {
    uuid: String,
    name: String,
    slot: i16,
    active: bool,
}

impl From<Device> for DeviceView {
    fn from(device: Device) -> Self {
        Self {
            uuid: device.uuid.to_string(),
            name: device.name,
            slot: device.slot,
            active: device.active,
        }
    }
}
