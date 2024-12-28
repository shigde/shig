use std::error::Error as StdError;
use std::fmt;

#[allow(dead_code)]
pub type FederationResult<T> = Result<T, FederationError>;

#[allow(dead_code)]
#[derive(Debug)]
pub struct FederationError {
    details: String,
}

#[allow(dead_code)]
impl FederationError {
    #[inline(always)]
    fn new(msg: String) -> FederationError {
        FederationError { details: msg }
    }
}

impl fmt::Display for FederationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "federation: {}", self.details)
    }
}
impl StdError for FederationError {
    fn description(&self) -> &str {
        &self.details
    }
}

impl From<diesel::r2d2::Error> for FederationError {
    fn from(e: diesel::r2d2::Error) -> Self {
        FederationError::new(e.to_string())
    }
}

impl From<diesel::r2d2::PoolError> for FederationError {
    fn from(e: diesel::r2d2::PoolError) -> Self {
        FederationError::new(e.to_string())
    }
}

impl From<diesel::result::Error> for FederationError {
    fn from(e: diesel::result::Error) -> Self {
        FederationError::new(e.to_string())
    }
}

impl From<Box<dyn StdError + Send + Sync>> for FederationError {
    fn from(e: Box<dyn StdError + Send + Sync>) -> Self {
        FederationError::new(e.to_string())
    }
}

impl From<String> for FederationError {
    fn from(e: String) -> Self {
        FederationError::new(e.clone())
    }
}
