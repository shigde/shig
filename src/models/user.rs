pub mod channel;

use crate::db::users::delete::delete_user_by_id;
use crate::db::DbPool;
use crate::models::auth::session::Principal;
use crate::models::error::ApiError;
use crate::util::domain::split_domain_name;
use actix_web::web;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub name: String,
    pub domain: String,
    pub channel_actor: String,
    pub role: i32,
}

impl User {
    pub fn from_principal(principal: Principal) -> User {
        let (name, domain) = split_domain_name(principal.name.as_str());
        User {
            name,
            domain,
            channel_actor: principal.channel_actor,
            role: principal.user_role_id,
        }
    }
}

pub fn delete_by_principal(pool: &web::Data<DbPool>, principal: Principal) -> Result<(), ApiError> {
    let mut conn = pool.get()?;
    delete_user_by_id(&mut conn, principal.id)?;
    Ok(())
}
