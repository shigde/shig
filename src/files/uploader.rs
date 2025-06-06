use crate::files::error::{FileError, FileErrorKind, FileResult};
use crate::files::FilesConfig;
use actix_multipart::form::tempfile::TempFile;
use mime::Mime;
use std::path::PathBuf;

pub struct ImageUpload {
    pub filename: String,
    pub height: i32,
    pub width: i32,
    pub file_url: String,
}

pub struct Uploader {
    cfg: FilesConfig,
    max_file_size: usize,
}

impl Uploader {
    pub fn new(cfg: FilesConfig) -> Uploader {
        Uploader {
            cfg,
            max_file_size: 1024 * 1024 * 10, // 10MB
        }
    }
    pub fn upload(
        &self,
        file: &TempFile,
        file_upload_name: String,
        destination: String,
    ) -> FileResult<ImageUpload> {
        // Reject empty files
        let filetype: Option<Mime> = file.content_type.clone();
        if filetype.is_none() {
            return Err(FileError::new(
                "no content type".to_string(),
                FileErrorKind::BadArgument,
            ));
        }

        // Reject unsupported file types
        let file_extension = match file.content_type.clone().unwrap().subtype() {
            mime::PNG => "png",
            mime::JPEG => "jpeg",
            mime::GIF => "gif",
            _ => {
                return Err(FileError::new(
                    "no legal content type".to_string(),
                    FileErrorKind::BadArgument,
                ))
            }
        };

        // Reject malformed requests
        match file.size {
            0 => {
                return Err(FileError::new(
                    "Empty file".to_string(),
                    FileErrorKind::BadArgument,
                ))
            }
            length if length > self.max_file_size => {
                return Err(FileError::new(
                    format!(
                        "The uploaded file is too large. Maximum size is {} bytes.",
                        self.max_file_size
                    ),
                    FileErrorKind::BadArgument,
                ));
            }
            _ => {}
        };

        // Get the temp file
        let temp_file_path = file.file.path();

        // Build the new file path
        let file_name: String = format!("{}.{}", file_upload_name, file_extension);
        let file_sub_path = format!("{}/{}", self.cfg.htdocs, destination);
        let mut file_path = PathBuf::from(file_sub_path);
        file_path.push(&sanitize_filename::sanitize(&file_name));

        // Remove the file if already exists
        match std::fs::remove_file(file_path.clone()) {
            Ok(()) => "ok",
            Err(_) => "no file",
        };

        // Move the tmp file to the new file path
        match std::fs::rename(temp_file_path, file_path) {
            Ok(_) => Ok(ImageUpload {
                filename: file_name.to_string(),
                height: 0,
                width: 0,
                file_url: format!("{}/{}", destination, file_name),
            }),
            Err(_) => Err(FileError::new(
                "Internal".to_string(),
                FileErrorKind::Internal,
            )),
        }
    }
}
