use crate::files::error::{FileError, FileErrorKind, FileResult};
use crate::files::FilesConfig;
use crate::models::auth::session::Principal;
use actix_multipart::form::tempfile::TempFile;
use mime::{Mime, IMAGE_GIF, IMAGE_JPEG, IMAGE_PNG};
use std::path::PathBuf;

pub struct ImageUpload {
    pub filename: String,
    pub height: i32,
    pub width: i32,
    pub file_url: String,
}

pub struct Uploader {
    cfg: FilesConfig,
    legal_filetypes: [Mime; 3],
    max_file_size: u64,
}

impl Uploader {
    pub fn new(cfg: FilesConfig) -> Uploader {
        Uploader {
            cfg,
            legal_filetypes: [IMAGE_PNG, IMAGE_JPEG, IMAGE_GIF],
            max_file_size: 1024 * 1024 * 10, // 10MB
        }
    }
    pub fn upload(
        &self,
        principal: Principal,
        file: &TempFile,
        destination: String,
    ) -> FileResult<ImageUpload> {
        // Reject empty files
        let filetype: Option<Mime> = *file.content_type;
        if filetype.is_none() {
            return Err(FileError::new(
                "no content type".to_string(),
                FileErrorKind::BadArgument,
            ));
        }

        // Reject unsupported file types
        if !self.legal_filetypes.contains(&filetype.unwrap()) {
            return Err(FileError::new(
                "no legal content type".to_string(),
                FileErrorKind::BadArgument,
            ));
        }

        let file_extension: String = get_image_extension(filetype.clone().unwrap_or(IMAGE_JPEG));

        // Reject malformed requests
        match file.size {
            0 => {
                return Err(FileError::new(
                    "Empty file".to_string(),
                    FileErrorKind::BadArgument,
                ))
            }
            length if length > self.max_file_size.try_into().unwrap() => {
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
        let file_name: String = format!("{}.{}", principal.user_uuid, file_extension);
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

fn get_image_extension(mime: Mime) -> String {
    match mime {
        IMAGE_JPEG => "jpg".to_string(),
        IMAGE_PNG => "gif".to_string(),
        IMAGE_PNG => "png".to_string(),
        _ => "jpg".to_string(),
    }
}
