use derive_more::Display;

pub type WorkerResult<T> = Result<T, WorkerError>;

#[derive(Debug, Display)]
pub enum WorkerError {
    #[display(fmt = "worker already exists")]
    AlreadyExists,
    #[display(fmt = "worker not found")]
    NotFound,
    #[display(fmt = "failed to spawn process {}", _0)]
    Spawn(String),
    #[display(fmt = "Process failed: {}", _0)]
    ProcessFailed(String),
    #[display(fmt = "Filo error {}", _0)]
    Filo(String),
}
