use crate::sfu::media::error::MediaError;
use derive_more::Display;

pub type SfuResult<T> = Result<T, SfuError>;

#[derive(Debug, Display)]
pub enum SfuError {
    #[allow(dead_code)]
    LobbyAlreadyStarted(),
    LobbyNotExists(),

    // Error by sendin msg to a lobby
    LobbyError(LobbyError),
    LobbyMailboxError(actix::MailboxError),
}

pub type LobbyResult<T> = Result<T, LobbyError>;

#[derive(Debug, Display)]
pub enum LobbyError {
    PeerInternalError(PeerError),
    PeerAlreadyExists(),
    MailboxError(actix::MailboxError),
}

pub type PeerResult<T> = Result<T, PeerError>;

#[derive(Debug, Display)]
pub enum PeerError {
    #[display(fmt = "Peer internal error: {}", _0)]
    InternalMedia(MediaError),
    #[allow(dead_code)]
    PeerAlreadyStarted(),
}

impl From<MediaError> for PeerError {
    fn from(e: MediaError) -> Self {
        PeerError::InternalMedia(e)
    }
}
