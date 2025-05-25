use crate::db::stream_meta_data::create::insert_new_stream_meta_data;
use crate::db::stream_meta_data::update::update_stream_meta_data;
use crate::db::stream_thumbnails::create::insert_new_stream_thumbnail;
use crate::db::stream_thumbnails::update::update_stream_thumbnail;
use crate::db::streams::create::insert_new_stream;
use crate::db::streams::read::find_stream_by_uuid;
use crate::db::streams::update::update_stream;
use crate::db::DbPool;
use crate::files::uploader::{ImageUpload, Uploader};
use crate::files::FilesConfig;
use crate::models::auth::session::Principal;
use crate::models::error::ApiError;
use crate::models::user::stream::Stream;
use crate::models::user::stream_thumbnail::StreamThumbnail;
use actix_multipart::form::{json::Json as MpJson, tempfile::TempFile, MultipartForm};
use actix_web::web;

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
        // Check if user is the owner of the stream
        if principal.user_uuid != self.stream.owner_uuid {
            return Err(ApiError::Unauthorized {
                error_message: "unauthorized".to_string(),
            });
        }

        let mut thumbnail_img: Option<ImageUpload> = None;
        let mut conn = pool.get()?;
        let stream_uuid = uuid::Uuid::new_v4().to_string();

        // Upload the Thumbnail if was one uploaded in the form
        if self.file.size != 0 {
            // Upload file
            let image_uploader = Uploader::new(cgf.get_ref().clone());
            let img = image_uploader.upload(
                &self.file,
                stream_uuid.to_owned(),
                "thumbnail".to_string(),
            )?;
            thumbnail_img = Some(img);
        }

        let stream_dao = insert_new_stream(
            &mut conn,
            stream_uuid.as_str(),
            principal.id,
            principal.channel_id,
            self.stream.title.as_str(),
            Some(self.stream.description.as_str()),
            Some(self.stream.support.as_str()),
            self.stream.date.to_owned(),
            self.stream.licence,
            self.stream.is_repeating,
            Some(self.stream.repeat),
            self.stream.is_public,
        )?;
        
        // Insert stream Meta Data
        let new_meta_data = self.stream.meta_data.build_insert_dao(stream_dao.id);
        insert_new_stream_meta_data(&mut conn, new_meta_data)?;

        // Build Stream Response Object
        let mut stream_response = self.stream.clone();
        stream_response.uuid = stream_dao.uuid.clone();

        // Insert Thumbnail if was one uploaded
        match thumbnail_img {
            None => {}
            Some(thumbnail_img) => {
                let thumbnail_dao =
                    StreamThumbnail::build_insert_dao(stream_dao.id, &thumbnail_img);
                insert_new_stream_thumbnail(&mut conn, thumbnail_dao.clone())?;
                let url = thumbnail_dao.file_url;
                
                // Update the stream response object with the new thumbnail url
                stream_response.thumbnail = url.to_string();
            }
        }

        Ok(stream_response)
    }

    pub fn update(
        &self,
        pool: &web::Data<DbPool>,
        principal: Principal,
        cgf: &web::Data<FilesConfig>,
    ) -> Result<Stream, ApiError> {
        let mut conn = pool.get()?;
        let stream_uuid = self.stream.uuid.clone();
        let current_stream_dao = find_stream_by_uuid(&mut conn, stream_uuid.clone())?;
        let stream_id = current_stream_dao.stream.id;

        // Check if user is the owner of the stream
        if principal.id != current_stream_dao.stream.user_id {
            return Err(ApiError::Unauthorized {
                error_message: "unauthorized".to_string(),
            });
        }

        let mut thumbnail_img: Option<ImageUpload> = None;

        // Upload the Thumbnail if was one uploaded in the form
        if self.file.size != 0 {
            // Upload file
            let image_uploader = Uploader::new(cgf.get_ref().clone());
            let img = image_uploader.upload(
                &self.file,
                stream_uuid.to_owned(),
                "thumbnail".to_string(),
            )?;
            thumbnail_img = Some(img);
        }

        update_stream(
            &mut conn,
            stream_id,
            self.stream.title.as_str(),
            Some(self.stream.description.as_str()),
            Some(self.stream.support.as_str()),
            self.stream.date.to_owned(),
            self.stream.start_time.to_owned(),
            self.stream.end_time.to_owned(),
            self.stream.licence,
            self.stream.is_repeating,
            Some(self.stream.repeat),
            self.stream.is_public,
            self.stream.is_live,
        )?;

        // Update stream Meta Data
        let meta_data_dao = self.stream.meta_data.build_update_dao(stream_id);
        update_stream_meta_data(&mut conn, current_stream_dao.meta_data.id, meta_data_dao)?;

        // Build Stream Response Object
        let mut stream_response = self.stream.clone();
        stream_response.uuid = self.stream.uuid.clone();

        // Update Thumbnail if was one uploaded
        match thumbnail_img {
            None => {}
            Some(thumbnail_img) => {
                let url: &str;
                match current_stream_dao.thumbnail {
                    // If first time of uploaded then insert new thumbnail
                    None => {
                        let thumbnail_dao =
                            StreamThumbnail::build_insert_dao(stream_id, &thumbnail_img);
                        insert_new_stream_thumbnail(&mut conn, thumbnail_dao.clone())?;
                        url = thumbnail_dao.file_url;
                    }
                    // If already exist, then update
                    Some(current_image) => {
                        let thumbnail_dao = StreamThumbnail::build_update_dao(&thumbnail_img);
                        update_stream_thumbnail(
                            &mut conn,
                            current_image.id,
                            thumbnail_dao.clone(),
                        )?;
                        url = thumbnail_dao.file_url;
                    }
                };

                // Update the stream response object with the new thumbnail url
                stream_response.thumbnail = url.to_string();
            }
        };

        Ok(stream_response)
    }
}
