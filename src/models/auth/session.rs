use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Principal {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub user_uuid: String,
    pub user_role_id: i32,
    pub user_actor: String,
    pub channel_actor: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Session {
    pub principal: Principal,
}

impl Session {
    pub fn create(user: crate::db::sessions::Principal) -> Self {
        Session {
            principal: Principal {
                id: user.id,
                name: user.name,
                email: user.email,
                user_uuid: user.user_uuid,
                user_role_id: user.user_role_id,
                user_actor: user.user_actor,
                channel_actor: user.channel_actor,
            },
        }
    }
}
