use crate::worker::error::WorkerError;
use derive_more::Display;
use std::net::AddrParseError;

pub type RelayResult<T> = Result<T, RelayError>;

#[derive(Debug, Display)]
pub enum RelayError {
    #[display(fmt = "Could not allocate port for relay")]
    PortAllocationError(),
    #[display(fmt = "Media stream was already started")]
    MediaStreamAlreadyStarted(),
    #[display(fmt = "Media stream was not started")]
    MediaStreamNotStarted(),
    #[display(fmt = "Worker mailbox error {}", _0)]
    WorkerMailboxError(String),
    #[display(fmt = "Worker error {}", _0)]
    WorkerError(WorkerError),
    #[display(fmt = "Invalid address {}", _0)]
    InvalidAddress(AddrParseError),
    #[display(fmt = "No input track {}", _0)]
    NoInputTrack(String),
    #[allow(dead_code)]
    #[display(fmt = "Publishing error {}", _0)]
    PublisherError(String),
    #[display(fmt = "Start ffmpeg process failed error {}", _0)]
    StartProcessFailed(String),
    #[display(fmt = "Unauthorized: {}", _0)]
    Unauthorized(String),
    #[allow(dead_code)]
    #[display(fmt = "Cmaf split error: {}", _0)]
    CmafSplit(String),
    #[allow(dead_code)]
    #[display(fmt = "Cmaf preparation error: {}", _0)]
    CmafPreparation(String),
    #[allow(dead_code)]
    #[display(fmt = "Cmaf write error: {}", _0)]
    CmafWrite(String),
    #[allow(dead_code)]
    #[display(fmt = "Cmaf publisher error: {}", _0)]
    CmafPublisher(String),
    #[display(fmt = "Rtp forwarder error: {}", _0)]
    RtpForwarder(String),
}

