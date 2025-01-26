use crate::db::error::{DbError, DbErrorKind};
use crate::models::http::response::Body;
use actix_web::{
    error,
    http::{header::ContentType, StatusCode},
    HttpResponse,
};
use derive_more::{Display, Error};
use log::error;
use serde::de::StdError;
use crate::models::http::{MESSAGE_INTERNAL_SERVER_ERROR, MESSAGE_NOT_FOUND};

#[derive(Debug, Display, Error)]
pub enum ApiError {
    #[display(fmt = "{error_message}")]
    Unauthorized { error_message: String },

    #[display(fmt = "{error_message}")]
    InternalServerError { error_message: String },

    #[allow(dead_code)]
    #[display(fmt = "{error_message}")]
    BadRequest { error_message: String },

    #[display(fmt = "{error_message}")]
    NotFound { error_message: String },
}

impl error::ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match *self {
            ApiError::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
            ApiError::InternalServerError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            ApiError::NotFound { .. } => StatusCode::NOT_FOUND,
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
        error!("{}", e );
        match e.kind() {
            DbErrorKind::NotFound => ApiError::NotFound { error_message: MESSAGE_NOT_FOUND.to_string() },
            _ => ApiError::InternalServerError { error_message: MESSAGE_INTERNAL_SERVER_ERROR.to_string() },
        }
    }
}

impl From<diesel::r2d2::PoolError> for ApiError {
    fn from(e: diesel::r2d2::PoolError) -> Self {
        error!("{}", e );
        ApiError::InternalServerError { error_message: MESSAGE_INTERNAL_SERVER_ERROR.to_string() }
    }
}

impl From<Box<dyn StdError + Send + Sync>> for ApiError {
    fn from(e: Box<dyn StdError + Send + Sync>) -> Self {
        error!("{}", e );
        if e.to_string().contains("Record not found") {
            return ApiError::NotFound { error_message: MESSAGE_NOT_FOUND.to_string() }
        }
        ApiError::InternalServerError { error_message: MESSAGE_INTERNAL_SERVER_ERROR.to_string() }
    }
}

impl From<Box<dyn StdError>> for ApiError {
    fn from(e: Box<dyn StdError>) -> Self {
        error!("{}", e );
        if e.to_string().contains("Record not found") {
            return ApiError::NotFound { error_message: MESSAGE_NOT_FOUND.to_string() }
        }
        ApiError::InternalServerError { error_message: MESSAGE_INTERNAL_SERVER_ERROR.to_string() }
    }
}
