use derive_more::Display;

pub type RelayResult<T> = Result<T, RelayError>;

#[derive(Debug, Display)]
pub enum RelayError {
    #[display(fmt = "Media stream was already started")]
    MediaStreamAlreadyStarted(),
    #[display(fmt = "Media stream was not started")]
    MediaStreamNotStarted(),
    #[display(fmt = "No input track {}", _0)]
    NoInputTrack(String),
    #[display(fmt = "Publishing error {}", _0)]
    PublisherError(String),
    #[display(fmt = "Unauthorized: {}", _0)]
    Unauthorized(String),
}
