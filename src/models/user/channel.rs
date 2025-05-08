use crate::db::actor_images::create::insert_new_actor_image;
use crate::db::actor_images::read::{find_actor_image_by_actor_id};
use crate::db::actor_images::update::update_actor_image;
use crate::db::actor_images::ActorImageType;
use crate::db::channels::read::{find_channel_by_user_id, find_channel_by_uuid};
use crate::db::channels::update::update;
use crate::db::channels::update::ChannelUpdate;
use crate::db::channels::Channel as ChannelDb;
use crate::db::users::read::find_user_by_id;
use crate::db::users::User;
use crate::db::DbPool;
use crate::files::uploader::{ImageUpload, Uploader};
use crate::files::FilesConfig;
use crate::models::auth::session::Principal;
use crate::models::error::ApiError;
use actix_multipart::form::{json::Json as MpJson, tempfile::TempFile, MultipartForm};
use actix_web::web;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Channel {
    pub uuid: String,
    // user name@domain
    pub user: String,
    pub name: String,
    pub description: String,
    pub support: String,
    pub public: bool,
    pub banner_name: String,
}

impl Channel {

    pub fn fetch(pool: &web::Data<DbPool>, channel_uuid: String) -> Result<Channel, ApiError> {
        let mut conn = pool.get()?;
        let channel: ChannelDb = find_channel_by_uuid(&mut conn, channel_uuid)?;
        let user: User = find_user_by_id(&mut conn, channel.user_id)?;
        let banner = match find_actor_image_by_actor_id(&mut conn, channel.actor_id, ActorImageType::BANNER)
        {
            Ok(image) => image.file_url.unwrap_or("".to_string()),
            Err(_) => "".to_string(),
        };

        Ok(Channel {
            uuid: channel.uuid,
            user: user.name,
            name: channel.name,
            description: channel.description.unwrap_or("".to_string()),
            support: channel.support.unwrap_or("".to_string()),
            public: channel.public,
            banner_name: banner,
        })
    }

    pub fn get_channel_update(&self) -> ChannelUpdate {
        ChannelUpdate {
            name: self.name.clone(),
            description: Some(self.description.clone()),
            support: Some(self.support.clone()),
            public: self.public,
        }
    }
}

#[derive(Debug, MultipartForm)]
pub struct ChannelForm {
    pub channel: MpJson<Channel>,
    #[multipart(limit = "10MB")]
    pub file: TempFile,
}

impl ChannelForm {
    pub fn save(
        &self,
        pool: &web::Data<DbPool>,
        principal: Principal,
        cgf: &web::Data<FilesConfig>,
    ) -> Result<Channel, ApiError> {
        if principal.channel_uuid != self.channel.uuid {
            return Err(ApiError::Unauthorized {
                error_message: "unauthorized".to_string(),
            });
        }

        let image_upload: ImageUpload;
        let mut conn = pool.get()?;
        // Read current channel
        let current_channel: ChannelDb = find_channel_by_user_id(&mut conn, principal.id)?;

        // read current banner
        let mut banner_name = "".to_string();
        let banner_id: i32 = match find_actor_image_by_actor_id(
            &mut conn,
            current_channel.actor_id,
            ActorImageType::BANNER,
        ) {
            Ok(image) => {
                banner_name = image.file_url.unwrap_or("".to_string());
                image.id
            }
            Err(_) => -1,
        };

        // If the file not empty create a file
        if self.file.size != 0 {
            // Upload file
            let image_uploader = Uploader::new(cgf.get_ref().clone());
            image_upload = image_uploader.upload(
                principal,
                &self.file,
                ActorImageType::BANNER.value_as_str().to_string(),
            )?;

            // Save file
            if banner_id == -1 {
                insert_new_actor_image(
                    &mut conn,
                    image_upload.filename.as_str(),
                    image_upload.height,
                    image_upload.width,
                    image_upload.file_url.as_str(),
                    true,
                    ActorImageType::BANNER,
                    current_channel.actor_id,
                )?;
            } else {
                update_actor_image(
                    &mut conn,
                    banner_id,
                    image_upload.filename.as_str(),
                    image_upload.height,
                    image_upload.width,
                    image_upload.file_url.as_str(),
                    true,
                )?;
            }

            banner_name = image_upload.file_url.to_string();
        }

        // Update channel
        update(
            &mut conn,
            current_channel.id,
            self.channel.get_channel_update(),
        )?;

        let mut new_channel = self.channel.to_owned();
        new_channel.banner_name = banner_name;

        Ok(new_channel)
    }
}
