use std::error::{Error as StdError};
use std::fmt;

pub type FederationResult<T> = Result<T, FederationError>;

#[derive(Debug)]
pub enum FederationError {
    SignError,
    PersistenceError,
    InternalServerError,
}

impl fmt::Display for FederationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FederationError::SignError => f.write_str("SignError"),
            FederationError::PersistenceError => f.write_str("PersistenceError"),
            FederationError::InternalServerError => f.write_str("InternalError"),
        }
    }
}
impl StdError for FederationError {
    fn description(&self) -> &str {
        match *self {
            FederationError::SignError => "SignError",
            FederationError::PersistenceError => "Persistence error",
            FederationError::InternalServerError => "Internal error",
        }
    }
}

impl From<diesel::r2d2::Error> for FederationError {
    fn from(_: diesel::r2d2::Error) -> Self {
        FederationError::InternalServerError
    }
}

impl From<diesel::r2d2::PoolError> for FederationError {
    fn from(_: diesel::r2d2::PoolError) -> Self {
        FederationError::InternalServerError
    }
}

impl From<diesel::result::Error> for FederationError {
    fn from(_: diesel::result::Error) -> Self {
        FederationError::InternalServerError
    }
}

impl From<Box<dyn StdError + Send + Sync>> for FederationError {
    fn from(_: Box<dyn StdError + Send + Sync>) -> Self {
        FederationError::InternalServerError
    }
}
