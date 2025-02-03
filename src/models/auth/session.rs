use crate::db::users::User;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Principal {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub user_uuid: String,
    pub user_role_id: i32,
    pub actor_id: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Session {
    pub principal: Principal,
}

impl Session {
    pub fn create(user: User) -> Self {
        Session {
            principal: Principal {
                id: user.id,
                name: user.name,
                email: user.email,
                user_uuid: user.user_uuid,
                user_role_id: user.user_role_id,
                actor_id: user.actor_id,
            },
        }
    }
}
