use crate::sfu::peer::PeerId;
use crate::sfu::rtc::media_command::RtcError;
use derive_more::Display;

pub type SfuResult<T> = Result<T, SfuError>;

#[derive(Debug, Display)]
pub enum SfuError {
    LobbyNotExists(),
    LobbyError(LobbyError),
    LobbyMailboxError(actix::MailboxError),
    RtcMailboxError(actix::MailboxError),
    RtcError(RtcError),
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
    #[display(fmt = "RTC core unavailable: {}", _0)]
    RtcCoreUnavailable(String),
}

pub type PeerResult<T> = Result<T, PeerError>;

#[derive(Debug, Display)]
pub enum PeerError {
    #[display(fmt = "RTC error: {}", _0)]
    Rtc(String),
}

impl From<RtcError> for PeerError {
    fn from(error: RtcError) -> Self {
        Self::Rtc(error.to_string())
    }
}
