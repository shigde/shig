use crate::sfu::media::connector::ConnectorType;
use crate::sfu::media::data_channel::DataChannelMsg;
use crate::sfu::media::{Media, MediaId};
use crate::sfu::peer::PeerId;
use actix::Message;

/// Internal meda messages for peer
#[derive(Message)]
#[rtype(result = "()")]
pub enum MediaMessage {
    Connected(ConnectorType),
    Disconnected(ConnectorType),
    AddMedia(Media),
    RemoveMedia(PeerId, MediaId),
}
