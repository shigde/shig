use crate::db::stream_thumbnails::create::NewStreamThumbnail;
use crate::db::stream_thumbnails::update::StreamThumbnailUpdate;
use crate::files::uploader::ImageUpload;
use chrono::Utc;

pub struct StreamThumbnail {}

impl StreamThumbnail {
    pub fn build_insert_dao(stream_id: i32, thumbnail: &ImageUpload) -> NewStreamThumbnail {
        NewStreamThumbnail {
            filename: thumbnail.filename.as_str(),
            height: thumbnail.height,
            width: thumbnail.width,
            file_url: thumbnail.file_url.as_str(),
            on_disk: true,
            stream_id,
            created_at: Utc::now().naive_utc(),
        }
    }

    pub fn build_update_dao(thumbnail: &ImageUpload) -> StreamThumbnailUpdate {
        StreamThumbnailUpdate {
            filename: thumbnail.filename.as_str(),
            height: thumbnail.height,
            width: thumbnail.width,
            file_url: thumbnail.file_url.as_str(),
            on_disk: true,
        }
    }
}
