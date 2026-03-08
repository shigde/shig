use crate::db::active_users::ActiveUser;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StreamParticipant {
    pub uuid: String,
}

impl StreamParticipant {
    pub fn from_active_user(active_user: ActiveUser) -> StreamParticipant {
        Self {
            uuid: active_user.user_uuid,
        }
    }
}
