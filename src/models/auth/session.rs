use crate::db::active_users::ActiveUser;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Principal {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub user_uuid: String,
    pub user_role_id: i32,
    pub user_actor_id: i32,
    pub channel_id: i32,
    pub channel_uuid: String,
    pub channel_actor_id: i32,
    pub avatar: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Session {
    pub principal: Principal,
}

impl Session {
    pub fn create(user: ActiveUser) -> Self {
        Session {
            principal: Principal {
                id: user.id,
                name: user.name,
                email: user.email,
                user_uuid: user.user_uuid,
                user_role_id: user.user_role_id,
                user_actor_id: user.user_actor_id,
                channel_id: user.channel_id,
                channel_uuid: user.channel_uuid,
                channel_actor_id: user.channel_actor_id,
                avatar: user.avatar,
            },
        }
    }
}
