use crate::sfu::rtc::media_command::{
    ApplyEndpointAnswer, ApplySfuEvent, CloseEndpoint, CreateEndpointOffer, NegotiateEndpoint,
    RtcError, RtcEvent, SetRtcEventSink, StopRtcCore,
};
use crate::sfu::rtc::media::{MediaEngine, SFUEvent};
use actix::{
    Actor, ActorContext, Addr, AsyncContext, Context, Handler, Message, Recipient, Running,
};
use bytes::BytesMut;
use rtc::shared::{TaggedBytesMut, TransportContext, TransportProtocol};
use sansio::Protocol;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::task::JoinHandle;

pub type RtcCoreId = u64;

const RECEIVE_BUFFER_SIZE: usize = 65_535;
const IDLE_TIMEOUT_POLL: Duration = Duration::from_millis(100);

/// Owns one RTC engine and drives it on a single Actix Arbiter.
pub struct RtcCoreActor {
    id: RtcCoreId,
    engine: MediaEngine,
    socket: Arc<UdpSocket>,
    advertised_addr: SocketAddr,
    event_sink: Option<Recipient<RtcEvent>>,
    timeout_generation: u64,
    udp_receiver: Option<JoinHandle<()>>,
}

impl RtcCoreActor {
    pub async fn bind(
        id: RtcCoreId,
        bind_addr: SocketAddr,
        advertised_addr: SocketAddr,
    ) -> io::Result<Self> {
        let socket = Arc::new(UdpSocket::bind(bind_addr).await?);
        Ok(Self::from_socket(id, socket, advertised_addr))
    }

    pub fn from_socket(id: RtcCoreId, socket: Arc<UdpSocket>, advertised_addr: SocketAddr) -> Self {
        Self {
            id,
            engine: MediaEngine::new(id, advertised_addr),
            socket,
            advertised_addr,
            event_sink: None,
            timeout_generation: 0,
            udp_receiver: None,
        }
    }

    pub fn id(&self) -> RtcCoreId {
        self.id
    }

    pub fn media_addr(&self) -> SocketAddr {
        self.advertised_addr
    }

    fn start_udp_receiver(&self, actor: Addr<Self>) -> JoinHandle<()> {
        let socket = Arc::clone(&self.socket);
        let local_addr = self.advertised_addr;

        actix::spawn(async move {
            let mut buffer = vec![0_u8; RECEIVE_BUFFER_SIZE];
            loop {
                match socket.recv_from(&mut buffer).await {
                    Ok((size, peer_addr)) => actor.do_send(InboundDatagram {
                        received_at: Instant::now(),
                        local_addr,
                        peer_addr,
                        payload: BytesMut::from(&buffer[..size]),
                    }),
                    Err(error) => {
                        log::warn!("RTC UDP receive stopped: local={local_addr} error={error}");
                        actor.do_send(UdpReceiverStopped);
                        break;
                    }
                }
            }
        })
    }

    fn drain(&mut self, ctx: &mut Context<Self>, return_events: bool) -> Vec<SFUEvent> {
        let _ = self.engine.poll_read();

        while let Some(output) = self.engine.poll_write() {
            let socket = Arc::clone(&self.socket);
            let peer_addr = output.transport.peer_addr;
            let payload = output.message.freeze();
            ctx.spawn(actix::fut::wrap_future(async move {
                if let Err(error) = socket.send_to(&payload, peer_addr).await {
                    log::warn!("RTC UDP send failed: peer={peer_addr} error={error}");
                }
            }));
        }

        let mut returned = Vec::new();
        while let Some(event) = self.engine.poll_event() {
            if return_events {
                returned.push(event);
            } else if let Some(sink) = &self.event_sink {
                sink.do_send(RtcEvent {
                    core_id: self.id,
                    event,
                });
            } else {
                log::warn!("dropping RTC event because no event sink is configured");
            }
        }

        self.schedule_timeout(ctx);
        returned
    }

    fn schedule_timeout(&mut self, ctx: &mut Context<Self>) {
        self.timeout_generation = self.timeout_generation.wrapping_add(1);
        let generation = self.timeout_generation;
        let deadline = self.engine.poll_timeout();
        let delay = deadline
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
            .unwrap_or(IDLE_TIMEOUT_POLL);

        ctx.notify_later(
            ProtocolTimeout {
                generation,
                protocol_timer_due: deadline.is_some(),
            },
            delay,
        );
    }
}

impl Actor for RtcCoreActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.udp_receiver = Some(self.start_udp_receiver(ctx.address()));
        self.schedule_timeout(ctx);
        log::info!(
            "RTC core started: core={} media={}",
            self.id,
            self.advertised_addr
        );
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> Running {
        if let Some(receiver) = self.udp_receiver.take() {
            receiver.abort();
        }
        if let Err(error) = self.engine.close() {
            log::warn!("RTC core shutdown failed: core={} error={error}", self.id);
        }
        Running::Stop
    }
}

impl Handler<ApplySfuEvent> for RtcCoreActor {
    type Result = Result<Vec<SFUEvent>, RtcError>;

    fn handle(&mut self, message: ApplySfuEvent, ctx: &mut Self::Context) -> Self::Result {
        self.engine
            .handle_event(message.0)
            .map_err(|error| RtcError(error.to_string()))?;
        Ok(self.drain(ctx, true))
    }
}

impl Handler<SetRtcEventSink> for RtcCoreActor {
    type Result = ();

    fn handle(&mut self, message: SetRtcEventSink, _ctx: &mut Self::Context) {
        self.event_sink = Some(message.0);
    }
}

impl Handler<NegotiateEndpoint> for RtcCoreActor {
    type Result = Result<String, RtcError>;

    fn handle(&mut self, message: NegotiateEndpoint, _ctx: &mut Self::Context) -> Self::Result {
        Err(RtcError(format!(
            "endpoint-based RTC negotiation is not implemented yet: {:?} (offer bytes: {})",
            message.endpoint_id,
            message.offer.len()
        )))
    }
}

impl Handler<CreateEndpointOffer> for RtcCoreActor {
    type Result = Result<String, RtcError>;

    fn handle(&mut self, message: CreateEndpointOffer, _ctx: &mut Self::Context) -> Self::Result {
        Err(RtcError(format!(
            "endpoint-based RTC offer creation is not implemented yet: {:?}",
            message.endpoint_id
        )))
    }
}

impl Handler<ApplyEndpointAnswer> for RtcCoreActor {
    type Result = Result<(), RtcError>;

    fn handle(&mut self, message: ApplyEndpointAnswer, _ctx: &mut Self::Context) -> Self::Result {
        Err(RtcError(format!(
            "endpoint-based RTC answer handling is not implemented yet: {:?} (answer bytes: {})",
            message.endpoint_id,
            message.answer.len()
        )))
    }
}

impl Handler<CloseEndpoint> for RtcCoreActor {
    type Result = Result<(), RtcError>;

    fn handle(&mut self, message: CloseEndpoint, _ctx: &mut Self::Context) -> Self::Result {
        log::debug!(
            "ignoring close for endpoint not yet attached to RTC core {}: {:?}, reason={}",
            self.id,
            message.endpoint_id,
            message.reason
        );
        Ok(())
    }
}

impl Handler<StopRtcCore> for RtcCoreActor {
    type Result = ();

    fn handle(&mut self, _message: StopRtcCore, ctx: &mut Self::Context) {
        ctx.stop();
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct InboundDatagram {
    received_at: Instant,
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
    payload: BytesMut,
}

impl Handler<InboundDatagram> for RtcCoreActor {
    type Result = ();

    fn handle(&mut self, message: InboundDatagram, ctx: &mut Self::Context) {
        let input = TaggedBytesMut {
            now: message.received_at,
            transport: TransportContext {
                local_addr: message.local_addr,
                peer_addr: message.peer_addr,
                transport_protocol: TransportProtocol::UDP,
                ecn: None,
            },
            message: message.payload,
        };

        if let Err(error) = self.engine.handle_read(input) {
            log::debug!(
                "RTC packet rejected: core={} local={} peer={} error={error}",
                self.id,
                message.local_addr,
                message.peer_addr
            );
        }
        self.drain(ctx, false);
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct ProtocolTimeout {
    generation: u64,
    protocol_timer_due: bool,
}

impl Handler<ProtocolTimeout> for RtcCoreActor {
    type Result = ();

    fn handle(&mut self, message: ProtocolTimeout, ctx: &mut Self::Context) {
        if message.generation != self.timeout_generation {
            return;
        }

        if message.protocol_timer_due {
            if let Err(error) = self.engine.handle_timeout(Instant::now()) {
                log::warn!("RTC timeout failed: core={} error={error}", self.id);
            }
            self.drain(ctx, false);
        } else {
            self.schedule_timeout(ctx);
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct UdpReceiverStopped;

impl Handler<UdpReceiverStopped> for RtcCoreActor {
    type Result = ();

    fn handle(&mut self, _message: UdpReceiverStopped, ctx: &mut Self::Context) {
        ctx.stop();
    }
}
