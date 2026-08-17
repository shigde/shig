use actix::{Message, Recipient};
use sansio_sfu::SFUEvent;
use std::fmt::{Display, Formatter};

use crate::sfu::endpoint::EndpointId;
use crate::sfu::lobby::LobbyId;
use crate::sfu::rtc::core_actor::{RtcCoreActor, RtcCoreId};
use actix::Addr;

/// Error returned at the actor boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtcError(pub String);

impl Display for RtcError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RtcError {}

/// Applies a signaling/control event to the Sans-I/O SFU.
///
/// Events synchronously produced while applying the command are returned to the
/// caller. Events produced later by UDP input or a protocol timeout are sent to
/// the configured [`SetRtcEventSink`] recipient.
#[derive(Message)]
#[rtype(result = "Result<Vec<SFUEvent>, RtcError>")]
pub struct ApplySfuEvent(pub SFUEvent);

/// An asynchronously produced SFU event.
#[derive(Message)]
#[rtype(result = "()")]
pub struct RtcEvent {
    pub core_id: RtcCoreId,
    pub event: SFUEvent,
}

/// Installs the control-plane recipient for asynchronous RTC events.
///
/// There is deliberately one sink: event delivery has a single owner and does
/// not require `SFUEvent` to be cloned.
#[derive(Message)]
#[rtype(result = "()")]
pub struct SetRtcEventSink(pub Recipient<RtcEvent>);

/// Assigns a lobby to one core. Repeated requests return the stable assignment.
#[derive(Message)]
#[rtype(result = "Result<RtcCoreAssignment, RtcError>")]
pub struct AssignLobby(pub LobbyId);

/// Removes the stable assignment after a lobby has fully stopped.
#[derive(Message)]
#[rtype(result = "()")]
pub struct ReleaseLobby(pub LobbyId);

/// Negotiates one domain endpoint with an SDP offer.
///
/// This is the stable actor boundary for the future endpoint-based RTC core.
#[derive(Message)]
#[rtype(result = "Result<String, RtcError>")]
pub struct NegotiateEndpoint {
    pub endpoint_id: EndpointId,
    pub offer: String,
}

#[derive(Message)]
#[rtype(result = "Result<String, RtcError>")]
pub struct CreateEndpointOffer {
    pub endpoint_id: EndpointId,
}

#[derive(Message)]
#[rtype(result = "Result<(), RtcError>")]
pub struct ApplyEndpointAnswer {
    pub endpoint_id: EndpointId,
    pub answer: String,
}

/// Removes one domain endpoint from the RTC core.
#[derive(Message)]
#[rtype(result = "Result<(), RtcError>")]
pub struct CloseEndpoint {
    pub endpoint_id: EndpointId,
    pub reason: String,
}

#[derive(Clone)]
pub struct RtcCoreAssignment {
    pub core_id: RtcCoreId,
    pub core: Addr<RtcCoreActor>,
    pub media_addr: std::net::SocketAddr,
}

/// Stops a core after the pool has stopped accepting new work.
#[derive(Message)]
#[rtype(result = "()")]
pub(crate) struct StopRtcCore;

/// Stops the pool and all of its core actors.
#[derive(Message)]
#[rtype(result = "()")]
pub struct StopRtcPool;
