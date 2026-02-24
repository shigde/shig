use crate::sfu::media::connector::ConnectorType;
use crate::sfu::media::{Media, MediaId};
use crate::sfu::peer::PeerId;
use actix::Message as ActorMessage;
use serde::{Deserialize, Serialize};

/// Internal meda messages for peer
#[derive(ActorMessage)]
#[rtype(result = "()")]
pub enum MediaMessage {
    Connected(ConnectorType),
    Disconnected(ConnectorType),
    #[allow(dead_code)]
    AddMedia(Media),
    #[allow(dead_code)]
    RemoveMedia(PeerId, MediaId),
}

