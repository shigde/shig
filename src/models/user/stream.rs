use crate::db::actor_images::create::insert_new_actor_image;
use crate::db::actor_images::read::find_actor_image_by_actor_id;
use crate::db::actor_images::update::update_actor_image;
use crate::db::actor_images::ActorImageType;
use crate::db::channels::read::find_channel_by_user_id;
use crate::db::channels::update::update;
use crate::db::DbPool;
use crate::files::uploader::{ImageUpload, Uploader};
use crate::files::FilesConfig;
use crate::models::auth::session::Principal;
use crate::models::error::ApiError;
use crate::models::user::stream_meta_data::StreamMetaData;
use actix_multipart::form::{json::Json as MpJson, tempfile::TempFile, MultipartForm};
use actix_web::web;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Stream {
    pub uuid: String,
    pub title: String,
    pub thumbnail: String,
    pub description: String,
    pub support: String,
    pub date: NaiveDateTime,
    pub start_time: Option<NaiveDateTime>,
    pub end_time: Option<NaiveDateTime>,
    pub viewer: i64,
    pub likes: i64,
    pub dislikes: i64,
    pub licence: i32,
    pub is_repeating: bool,
    pub repeat: i32,
    pub meta_data: StreamMetaData,
    pub is_live: bool,
    pub is_public: bool,
    pub owner_uuid: String,
    pub channel_uuid: String,
}

#[derive(Debug, MultipartForm)]
pub struct StreamForm {
    pub stream: MpJson<Stream>,
    #[multipart(limit = "10MB")]
    pub file: TempFile,
}

impl StreamForm {
    pub fn save(
        &self,
        pool: &web::Data<DbPool>,
        principal: Principal,
        cgf: &web::Data<FilesConfig>,
    ) -> Result<Stream, ApiError> {
        if principal.user_uuid != self.stream.owner_uuid {
            return Err(ApiError::Unauthorized {
                error_message: "unauthorized".to_string(),
            });
        }

        let image_upload: ImageUpload;
        let mut conn = pool.get()?;

        // Read current channel
        let current_channel: crate::db::channels::Channel =
            find_channel_by_user_id(&mut conn, principal.id)?;

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
