pub mod channel;
pub mod stream;
pub mod stream_preview;
pub mod stream_meta_data;
mod stream_thumbnail;
mod stream_form;

use crate::db::users::delete::delete_user_by_id;
use crate::db::DbPool;
use crate::models::auth::session::Principal;
use crate::models::error::ApiError;
use crate::util::domain::split_domain_name;
use actix_web::web;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub uuid: String,
    pub name: String,
    pub domain: String,
    pub channel_uuid: String,
    pub avatar: String,
    pub role: i32,
}

impl User {
    pub fn from_principal(principal: Principal) -> User {
        let (name, domain) = split_domain_name(principal.name.as_str());
        User {
            uuid: principal.user_uuid,
            name,
            domain,
            channel_uuid: principal.channel_uuid,
            avatar: principal.avatar.unwrap_or("".to_string()),
            role: principal.user_role_id,
        }
    }
}

pub fn delete_by_principal(pool: &web::Data<DbPool>, principal: Principal) -> Result<(), ApiError> {
    let mut conn = pool.get()?;
    delete_user_by_id(&mut conn, principal.id)?;
    Ok(())
}
