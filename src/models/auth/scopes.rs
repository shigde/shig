use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The authorization context carried by an authenticated session.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionScope {
    #[default]
    Account,
    Device(DeviceScope),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DeviceScope {
    pub device_id: Uuid,
    pub stream_id: i32,
    pub session_id: Uuid,
}

/// The access requirement declared when registering an API service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequireScope {
    /// The service can be called without a session.
    Public,
    /// An authenticated account acting with its own account permissions.
    Account,
    /// An authenticated device acting within its assigned stream.
    Device,
}

impl RequireScope {
    pub fn allows(self, scope: &SessionScope) -> bool {
        matches!(
            (self, scope),
            (Self::Account, SessionScope::Account) | (Self::Device, SessionScope::Device(_))
        )
    }
}
