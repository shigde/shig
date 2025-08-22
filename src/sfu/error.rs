use derive_more::{Display, Error};

pub type SfuResult<T> = Result<T, SfuError>;

#[derive(Debug, Display, Error)]
pub enum SfuError {
    LobbyAlreadyStarted(),
    LobbyNotExists(),

    // Error by sendin msg to a lobby
    LobbyError(LobbyError),
    LobbyMailboxError(actix::MailboxError),
}

pub type LobbyResult<T> = Result<T, LobbyError>;

#[derive(Debug, Display, Error)]
pub enum LobbyError {
    #[allow(dead_code)]
    PeerAlreadyStarted(),
}
