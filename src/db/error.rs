use std::fmt;
use serde::de::StdError;

pub type DbResult<T> = Result<T, DbError>;

#[derive(Debug)]
pub struct DbError {
    details: String,
}

impl DbError {
    fn new(msg: String) -> DbError {
        DbError { details: msg }
    }
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "db: {}", self.details)
    }
}
impl StdError for DbError {
    fn description(&self) -> &str {
        &self.details
    }
}

impl From<diesel::r2d2::Error> for DbError {
    fn from(e: diesel::r2d2::Error) -> Self {
        DbError::new(e.to_string())
    }
}

impl From<diesel::r2d2::PoolError> for DbError {
    fn from(e: diesel::r2d2::PoolError) -> Self {
        DbError::new(e.to_string())
    }
}

impl From<diesel::result::Error> for DbError {
    fn from(e: diesel::result::Error) -> Self {
        DbError::new(e.to_string())
    }
}

impl From<Box<dyn StdError + Send + Sync>> for DbError {
    fn from(e: Box<dyn StdError + Send + Sync>) -> Self {
        DbError::new(e.to_string())
    }
}

impl From<String> for DbError {
    fn from(e: String) -> Self {
        DbError::new(e.clone())
    }
}
