use crate::db::error::DbError;
use crate::federation::error::FederationError;
use std::error::Error as StdError;
use std::fmt;
use std::io::Error as IoError;

#[allow(dead_code)]
pub type SfuResult<T> = Result<T, SfuError>;

#[derive(Debug)]
pub struct SfuError {
    details: String,
}

impl SfuError {
    fn new(msg: String) -> SfuError {
        SfuError { details: msg }
    }
}

impl fmt::Display for SfuError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "server: {}", self.details)
    }
}
impl StdError for SfuError {
    fn description(&self) -> &str {
        &self.details
    }
}

impl From<IoError> for SfuError {
    fn from(e: IoError) -> Self {
        SfuError::new(e.to_string())
    }
}

impl From<diesel::r2d2::Error> for SfuError {
    fn from(e: diesel::r2d2::Error) -> Self {
        SfuError::new(e.to_string())
    }
}

impl From<diesel::r2d2::PoolError> for SfuError {
    fn from(e: diesel::r2d2::PoolError) -> Self {
        SfuError::new(e.to_string())
    }
}

impl From<diesel::result::Error> for SfuError {
    fn from(e: diesel::result::Error) -> Self {
        SfuError::new(e.to_string())
    }
}

impl From<Box<dyn StdError + Send + Sync>> for SfuError {
    fn from(e: Box<dyn StdError + Send + Sync>) -> Self {
        SfuError::new(e.to_string())
    }
}

impl From<openssl::error::ErrorStack> for SfuError {
    fn from(e: openssl::error::ErrorStack) -> Self {
        SfuError::new(e.to_string())
    }
}

impl From<FederationError> for SfuError {
    fn from(e: FederationError) -> Self {
        SfuError::new(e.to_string())
    }
}

impl From<DbError> for SfuError {
    fn from(e: DbError) -> Self {
        SfuError::new(e.to_string())
    }
}
