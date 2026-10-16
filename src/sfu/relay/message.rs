use crate::sfu::relay::error::RelayResult;
use crate::sfu::relay::RelayMediaStream;
use actix::Message;
use moq_relay::auth::Token as AuthToken;

#[derive(Message)]
#[rtype(result = " RelayResult<()>")]
pub struct StartRelayMediaStream {
    pub media_stream: RelayMediaStream,
    pub auth_token: AuthToken,
}

#[derive(Message)]
#[rtype(result = " RelayResult<()>")]
pub struct StopRelayMediaStream {}
