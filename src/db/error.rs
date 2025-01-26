use serde::de::StdError;
use std::fmt;

pub type DbResult<T> = Result<T, DbError>;

#[derive(Debug)]
pub struct DbError {
    details: String,
    kind: DbErrorKind,
}

#[derive(Debug)]
pub enum DbErrorKind {
    NotFound,
    Internal,
}

impl DbError {
    pub fn new(msg: String, kind: DbErrorKind) -> DbError {
        DbError { details: msg, kind }
    }

    pub fn kind(&self) -> &DbErrorKind {
        &self.kind
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
        DbError::new(e.to_string(), DbErrorKind::Internal)
    }
}

impl From<diesel::r2d2::PoolError> for DbError {
    fn from(e: diesel::r2d2::PoolError) -> Self {
        DbError::new(e.to_string(), DbErrorKind::Internal)
    }
}

impl From<diesel::result::Error> for DbError {
    fn from(e: diesel::result::Error) -> Self {
        match e {
            diesel::result::Error::NotFound => DbError::new(e.to_string(), DbErrorKind::NotFound),
            _ => DbError::new(e.to_string(), DbErrorKind::Internal),
        }
    }
}

impl From<Box<dyn StdError + Send + Sync>> for DbError {
    fn from(e: Box<dyn StdError + Send + Sync>) -> Self {
        if e.to_string().contains("Record not found") {
            return DbError::new(e.to_string(), DbErrorKind::NotFound)
        }
        DbError::new(e.to_string(), DbErrorKind::Internal)
    }
}

impl From<String> for DbError {
    fn from(e: String) -> Self {
        if e.to_string().contains("Record not found") {
            return DbError::new(e.to_string(), DbErrorKind::NotFound)
        }
        DbError::new(e.clone(), DbErrorKind::Internal)
    }
}
