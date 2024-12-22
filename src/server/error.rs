use std::error::Error as StdError;
use std::fmt;
use std::io::Error as IoError;

pub type ServerResult<T> = Result<T, ServerError>;

#[derive(Debug)]
pub enum ServerError {
    IOError,
    InternalServerError,
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ServerError::IOError => f.write_str("ConnectionError"),
            ServerError::InternalServerError => f.write_str("InternalServerError"),
        }
    }
}
impl StdError for ServerError {
    fn description(&self) -> &str {
        match *self {
            ServerError::IOError => "Connection error",
            ServerError::InternalServerError => "Internal server error",
        }
    }
}

impl From<IoError> for ServerError {
    fn from(_: IoError) -> Self {
        ServerError::IOError
    }
}

impl From<diesel::r2d2::Error> for ServerError {
    fn from(_: diesel::r2d2::Error) -> Self {
        ServerError::InternalServerError
    }
}

impl From<diesel::r2d2::PoolError> for ServerError {
    fn from(_: diesel::r2d2::PoolError) -> Self {
        ServerError::InternalServerError
    }
}

impl From<diesel::result::Error> for ServerError {
    fn from(_: diesel::result::Error) -> Self {
        ServerError::InternalServerError
    }
}

impl From<Box<dyn StdError + Send + Sync>> for ServerError {
    fn from(_: Box<dyn StdError + Send + Sync>) -> Self {
        ServerError::InternalServerError
    }
}


impl From<openssl::error::ErrorStack> for ServerError {
    fn from(_: openssl::error::ErrorStack) -> Self {
        ServerError::InternalServerError
    }
}
