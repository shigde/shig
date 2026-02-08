use crate::db::active_users::read::find_if_exist_active_user_by_uuid;
use crate::db::active_users::ActiveUser;
use crate::db::stream_friends::create::insert_new_stream_friend;
use crate::db::stream_friends::delete::delete_stream_friend_by_user_and_stream_uuid;
use crate::db::stream_friends::read::{
    find_active_stream_friend_by_uuids, find_all_active_stream_friends_by_stream_uuid,
};
use crate::db::streams::read::find_if_exists_stream_by_uuid_for_owner;
use crate::db::DbPool;
use crate::models::auth::session::Principal;
use crate::models::error::ApiError;
use crate::models::user::friend_role::FriendRole;
use crate::util::domain::split_domain_name;
use actix_web::web::Data;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StreamFriend {
    pub uuid: String,
    pub name: String,
    pub domain: String,
    pub channel_uuid: String,
    pub avatar: String,
    pub role: i32,
}

impl StreamFriend {
    pub fn from_active_user(active_user: ActiveUser) -> StreamFriend {
        let (name, domain) = split_domain_name(active_user.name.as_str());
        Self {
            uuid: active_user.user_uuid,
            name,
            domain,
            channel_uuid: active_user.channel_uuid,
            avatar: active_user.avatar.unwrap_or("".to_string()),
            role: active_user.user_role_id,
        }
    }

    pub(crate) fn fetch(
        pool: &Data<DbPool>,
        stream_uuid: String,
        friend_uuid: String,
    ) -> Result<Option<StreamFriend>, ApiError> {
        let mut conn = pool.get()?;
        let Some(stream_friend) = find_active_stream_friend_by_uuids(
            &mut conn,
            stream_uuid.as_str(),
            friend_uuid.as_str(),
        )?
        else {
            return Ok(None);
        };

        Ok(Some(StreamFriend::from_active_user(stream_friend)))
    }

    pub(crate) fn fetch_all(
        pool: &Data<DbPool>,
        stream_uuid: String,
    ) -> Result<Vec<StreamFriend>, ApiError> {
        let mut conn = pool.get()?;
        let active_users =
            find_all_active_stream_friends_by_stream_uuid(&mut conn, stream_uuid.as_str())?;

        let mut stream_friends = Vec::with_capacity(active_users.len());

        for user in active_users {
            let friend = StreamFriend::from_active_user(user);
            stream_friends.push(friend);
        }

        Ok(stream_friends)
    }

    pub(crate) fn create(
        pool: &Data<DbPool>,
        stream_uuid: String,
        friend_uuid: String,
        principal: Principal,
    ) -> Result<StreamFriend, ApiError> {
        let mut conn = pool.get()?;
        let Some(stream) =
            find_if_exists_stream_by_uuid_for_owner(&mut conn, stream_uuid.as_str(), principal.id)?
        else {
            return Err(ApiError::Forbidden {
                error_message: "Create stream friend is forbidden".to_string(),
            });
        };

        let Some(user) = find_if_exist_active_user_by_uuid(&mut conn, friend_uuid.as_str())? else {
            return Err(ApiError::NotFound {
                error_message: "User not exists".to_string(),
            });
        };

        let role = FriendRole::Guest;
        let _guest = insert_new_stream_friend(&mut conn, user.id, stream.id, role.into())?;

        Ok(StreamFriend::from_active_user(user))
    }

    pub(crate) fn delete(
        pool: &Data<DbPool>,
        stream_uuid: String,
        friend_uuid: String,
        principal: Principal,
    ) -> Result<(), ApiError> {
        let mut conn = pool.get()?;
        if let None =
            find_if_exists_stream_by_uuid_for_owner(&mut conn, stream_uuid.as_str(), principal.id)?
        {
            return Err(ApiError::Forbidden {
                error_message: "Deleting stream friend is forbidden".to_string(),
            });
        };

        let result = delete_stream_friend_by_user_and_stream_uuid(
            &mut conn,
            stream_uuid.as_str(),
            friend_uuid.as_str(),
        )?;

        if result == 1 {
            Ok(())
        } else {
            Err(ApiError::NotAcceptable { error_message: "Could not delete".to_string() })
        }
    }
}
