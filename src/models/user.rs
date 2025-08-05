pub mod channel;
pub mod stream;
pub mod stream_form;
pub mod stream_meta_data;
pub mod stream_preview;
pub mod stream_thumbnail;
pub mod lobby;

use crate::db::active_users::read::find_active_user_by_uuid;
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

    pub(crate) fn find_as_active(
        pool: &web::Data<DbPool>,
        user_uuid: &str,
    ) -> Result<Self, ApiError> {
        let mut conn = pool.get()?;
        let active_user = find_active_user_by_uuid(&mut conn, user_uuid)?;

        let (name, domain) = split_domain_name(active_user.name.as_str());

        Ok(Self {
            uuid: active_user.user_uuid,
            name,
            domain,
            channel_uuid: active_user.channel_uuid,
            avatar: active_user.avatar.unwrap_or("".to_string()),
            role: active_user.user_role_id,
        })
    }
}

pub fn delete_by_principal(pool: &web::Data<DbPool>, principal: Principal) -> Result<(), ApiError> {
    let mut conn = pool.get()?;
    delete_user_by_id(&mut conn, principal.id)?;
    Ok(())
}
