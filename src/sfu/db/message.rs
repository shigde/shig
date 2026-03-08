use actix::Message;

#[derive(Message)]
#[rtype(result = "Result<(), diesel::result::Error>")]
pub struct SetLobbyOnline {
    pub lobby_uuid: String,
}

#[derive(Message)]
#[rtype(result = "Result<(), diesel::result::Error>")]
pub struct SetLobbyOffline {
    pub lobby_uuid: String,
}

#[derive(Message)]
#[rtype(result = "Result<(), diesel::result::Error>")]
pub struct AddParticipant {
    pub lobby_uuid: String,
    pub stream_uuid: String,
    pub user_uuid: String,
}

#[derive(Message)]
#[rtype(result = "Result<(), diesel::result::Error>")]
pub struct RemoveParticipant {
    pub lobby_uuid: String,
    pub stream_uuid: String,
    pub user_uuid: String,
}