use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkerError {
    #[error("worker already exists")]
    AlreadyExists,
    #[error("worker not found")]
    NotFound,
    #[error("failed to spawn process: {0}")]
    Spawn(String),
    #[error("Process failed: {0}")]
    ProcessFailed(String)
}
