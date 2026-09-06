use crate::sfu::media::error::MediaError;
use crate::sfu::peer::PeerId;
use crate::sfu::rtc::RtcError;
use derive_more::Display;

pub type SfuResult<T> = Result<T, SfuError>;

#[derive(Debug, Display)]
pub enum SfuError {
    #[allow(dead_code)]
    LobbyAlreadyStarted(),
    LobbyNotExists(),

    // Error by sending msg to a lobby
    LobbyError(LobbyError),
    LobbyMailboxError(actix::MailboxError),
}

pub type LobbyResult<T> = Result<T, LobbyError>;

#[derive(Debug, Display)]
pub enum LobbyError {
    MailboxError(actix::MailboxError),
    PeerInternalError(PeerError),
    #[display(fmt = "Peer already exist: {}", _0)]
    PeerAlreadyExists(PeerId),
    #[display(fmt = "Peer not exist: {}", _0)]
    PeerNotExists(PeerId),
    #[display(fmt = "Streaming error: {}", _0)]
    StreamingError(String),
}

pub type PeerResult<T> = Result<T, PeerError>;

#[derive(Debug, Display)]
pub enum PeerError {
    #[display(fmt = "Peer internal error: {}", _0)]
    InternalMedia(MediaError),
    #[display(fmt = "Peer RTC error: {}", _0)]
    InternalRtc(String),
    #[allow(dead_code)]
    PeerAlreadyStarted(),
}

impl From<MediaError> for PeerError {
    fn from(e: MediaError) -> Self {
        PeerError::InternalMedia(e)
    }
}

impl From<RtcError> for PeerError {
    fn from(e: RtcError) -> Self {
        PeerError::InternalRtc(e.to_string())
    }
}
