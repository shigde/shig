use crate::db::error::DbError;
use crate::federation::error::FederationError;
use std::error::Error as StdError;
use std::fmt;
use std::io::Error as IoError;

pub type ServerResult<T> = Result<T, ServerError>;

#[derive(Debug)]
pub struct ServerError {
    details: String,
}

impl ServerError {
    fn new(msg: String) -> ServerError {
        ServerError { details: msg }
    }
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "server: {}", self.details)
    }
}
impl StdError for ServerError {
    fn description(&self) -> &str {
        &self.details
    }
}

impl From<IoError> for ServerError {
    fn from(e: IoError) -> Self {
        ServerError::new(e.to_string())
    }
}

impl From<diesel::r2d2::Error> for ServerError {
    fn from(e: diesel::r2d2::Error) -> Self {
        ServerError::new(e.to_string())
    }
}

impl From<diesel::r2d2::PoolError> for ServerError {
    fn from(e: diesel::r2d2::PoolError) -> Self {
        ServerError::new(e.to_string())
    }
}

impl From<diesel::result::Error> for ServerError {
    fn from(e: diesel::result::Error) -> Self {
        ServerError::new(e.to_string())
    }
}

impl From<Box<dyn StdError + Send + Sync>> for ServerError {
    fn from(e: Box<dyn StdError + Send + Sync>) -> Self {
        ServerError::new(e.to_string())
    }
}

impl From<openssl::error::ErrorStack> for ServerError {
    fn from(e: openssl::error::ErrorStack) -> Self {
        ServerError::new(e.to_string())
    }
}

impl From<FederationError> for ServerError {
    fn from(e: FederationError) -> Self {
        ServerError::new(e.to_string())
    }
}

impl From<DbError> for ServerError {
    fn from(e: DbError) -> Self {
        ServerError::new(e.to_string())
    }
}
