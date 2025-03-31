use crate::db::error::{DbError, DbErrorKind};
use crate::files::error::{FileError, FileErrorKind};
use crate::models::http::response::Body;
use crate::models::http::{MESSAGE_INTERNAL_SERVER_ERROR, MESSAGE_NOT_FOUND};
use actix_web::{
    error,
    http::{header::ContentType, StatusCode},
    HttpResponse, ResponseError,
};
use derive_more::{Display, Error};
use log::error;
use serde::de::StdError;

#[derive(Debug, Display, Error)]
pub enum ApiError {
    #[display(fmt = "{error_message}")]
    Unauthorized { error_message: String },

    #[allow(dead_code)]
    #[display(fmt = "{error_message}")]
    Forbidden { error_message: String },

    #[display(fmt = "{error_message}")]
    Conflict { error_message: String },

    #[display(fmt = "{error_message}")]
    InternalServerError { error_message: String },

    #[allow(dead_code)]
    #[display(fmt = "{error_message}")]
    BadRequest { error_message: String },

    #[display(fmt = "{error_message}")]
    NotFound { error_message: String },

    #[display(fmt = "{error_message}")]
    NotAcceptable { error_message: String },
}

impl ApiError {
    pub fn is_message(&self, message: &str) -> bool {
        self.to_string().ends_with(message)
    }

    pub fn is_status_code(&self, status_code: StatusCode) -> bool {
        self.status_code() == status_code
    }
}

impl error::ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match *self {
            ApiError::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
            ApiError::InternalServerError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            ApiError::NotFound { .. } => StatusCode::NOT_FOUND,
            ApiError::Forbidden { .. } => StatusCode::FORBIDDEN,
            ApiError::Conflict { .. } => StatusCode::CONFLICT,
            ApiError::NotAcceptable { .. } => StatusCode::NOT_ACCEPTABLE,
        }
    }
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .json(Body::new(&self.to_string(), String::from("")))
    }
}

impl From<DbError> for ApiError {
    fn from(e: DbError) -> Self {
        error!("{}", e);
        match e.kind() {
            DbErrorKind::NotFound => ApiError::NotFound {
                error_message: MESSAGE_NOT_FOUND.to_string(),
            },
            DbErrorKind::AlreadyExists => ApiError::Conflict {
                error_message: e.to_string(),
            },
            _ => ApiError::InternalServerError {
                error_message: MESSAGE_INTERNAL_SERVER_ERROR.to_string(),
            },
        }
    }
}

impl From<diesel::r2d2::PoolError> for ApiError {
    fn from(e: diesel::r2d2::PoolError) -> Self {
        error!("{}", e);
        ApiError::InternalServerError {
            error_message: MESSAGE_INTERNAL_SERVER_ERROR.to_string(),
        }
    }
}

impl From<Box<dyn StdError + Send + Sync>> for ApiError {
    fn from(e: Box<dyn StdError + Send + Sync>) -> Self {
        error!("{}", e);
        if e.to_string().contains("Record not found") {
            return ApiError::NotFound {
                error_message: MESSAGE_NOT_FOUND.to_string(),
            };
        }
        ApiError::InternalServerError {
            error_message: MESSAGE_INTERNAL_SERVER_ERROR.to_string(),
        }
    }
}

impl From<Box<dyn StdError>> for ApiError {
    fn from(e: Box<dyn StdError>) -> Self {
        error!("{}", e);
        if e.to_string().contains("Record not found") {
            return ApiError::NotFound {
                error_message: MESSAGE_NOT_FOUND.to_string(),
            };
        }
        ApiError::InternalServerError {
            error_message: MESSAGE_INTERNAL_SERVER_ERROR.to_string(),
        }
    }
}

impl From<FileError> for ApiError {
    fn from(e: FileError) -> Self {
        match e.kind {
            FileErrorKind::BadArgument => ApiError::BadRequest {
                error_message: e.details.to_string(),
            },
            _ => ApiError::InternalServerError {
                error_message: MESSAGE_INTERNAL_SERVER_ERROR.to_string(),
            },
        }
    }
}
