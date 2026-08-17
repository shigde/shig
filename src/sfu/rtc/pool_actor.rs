use crate::sfu::config::{RtcAssignmentStrategy, SfuConfig};
use crate::sfu::lobby::LobbyId;
use crate::sfu::rtc::core_actor::{RtcCoreActor, RtcCoreId};
use crate::sfu::rtc::media_command::{
    AssignLobby, ReleaseLobby, RtcCoreAssignment, RtcError, StopRtcCore, StopRtcPool,
};
use actix::{Actor, ActorContext, Addr, Arbiter, Context, Handler, Running};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket as StdUdpSocket};
use std::sync::Arc;
use tokio::net::UdpSocket;

struct RtcCoreHandle {
    id: RtcCoreId,
    media_addr: SocketAddr,
    actor: Addr<RtcCoreActor>,
    lobby_count: usize,
    arbiter: Option<Arbiter>,
}

/// Configurable supervisor and control-plane router for RTC media cores.
pub struct RtcPoolActor {
    assignment_strategy: RtcAssignmentStrategy,
    cores: Vec<RtcCoreHandle>,
    lobby_assignments: HashMap<LobbyId, usize>,
    next_round_robin: usize,
}

impl RtcPoolActor {
    /// Binds all core sockets, starts the core actors, and starts the pool actor.
    ///
    /// When `dedicated_threads` is enabled, every core gets an independent Actix
    /// Arbiter and therefore an independent OS thread and Tokio reactor.
    pub fn launch(config: &SfuConfig) -> Result<Addr<Self>, RtcError> {
        let layout = RtcCoreLayout::from_config(config)?;
        let mut cores = Vec::with_capacity(layout.core_count);

        for index in 0..layout.core_count {
            let id = index as RtcCoreId;
            let port = layout.base_port + index as u16;
            let bind_addr = SocketAddr::new(layout.bind_ip, port);
            let media_addr = SocketAddr::new(layout.advertised_ip, port);
            let socket = StdUdpSocket::bind(bind_addr).map_err(|error| {
                RtcError(format!(
                    "failed to bind RTC core {id} at {bind_addr}: {error}"
                ))
            })?;
            socket
                .set_nonblocking(true)
                .map_err(|error| RtcError(format!("failed to configure RTC core {id}: {error}")))?;

            let (actor, arbiter) = if config.dedicated_threads {
                let arbiter = Arbiter::new();
                let actor = RtcCoreActor::start_in_arbiter(&arbiter.handle(), move |_ctx| {
                    let socket = UdpSocket::from_std(socket)
                        .expect("validated non-blocking RTC socket must attach to core reactor");
                    RtcCoreActor::from_socket(id, Arc::new(socket), media_addr)
                });
                (actor, Some(arbiter))
            } else {
                let socket = UdpSocket::from_std(socket).map_err(|error| {
                    RtcError(format!("failed to attach RTC core {id} socket: {error}"))
                })?;
                (
                    RtcCoreActor::from_socket(id, Arc::new(socket), media_addr).start(),
                    None,
                )
            };

            cores.push(RtcCoreHandle {
                id,
                media_addr,
                actor,
                lobby_count: 0,
                arbiter,
            });
        }

        Ok(Self {
            assignment_strategy: config.assignment,
            cores,
            lobby_assignments: HashMap::new(),
            next_round_robin: 0,
        }
        .start())
    }

    fn assign_lobby(&mut self, lobby_id: LobbyId) -> Result<RtcCoreAssignment, RtcError> {
        if let Some(&index) = self.lobby_assignments.get(&lobby_id) {
            return Ok(self.assignment_for(index));
        }

        let index = match self.assignment_strategy {
            RtcAssignmentStrategy::RoundRobin => {
                let index = self.next_round_robin % self.cores.len();
                self.next_round_robin = self.next_round_robin.wrapping_add(1);
                index
            }
            RtcAssignmentStrategy::LeastLoaded => self
                .cores
                .iter()
                .enumerate()
                .min_by_key(|(_, core)| (core.lobby_count, core.id))
                .map(|(index, _)| index)
                .ok_or_else(|| RtcError("RTC pool has no cores".to_owned()))?,
        };

        self.lobby_assignments.insert(lobby_id, index);
        self.cores[index].lobby_count += 1;
        Ok(self.assignment_for(index))
    }

    fn assignment_for(&self, index: usize) -> RtcCoreAssignment {
        let core = &self.cores[index];
        RtcCoreAssignment {
            core_id: core.id,
            core: core.actor.clone(),
            media_addr: core.media_addr,
        }
    }
}

impl Actor for RtcPoolActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        log::info!("RTC pool started with {} core(s)", self.cores.len());
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> Running {
        for core in &self.cores {
            core.actor.do_send(StopRtcCore);
        }
        for core in &self.cores {
            if let Some(arbiter) = &core.arbiter {
                arbiter.stop();
            }
        }
        Running::Stop
    }
}

impl Handler<AssignLobby> for RtcPoolActor {
    type Result = Result<RtcCoreAssignment, RtcError>;

    fn handle(&mut self, message: AssignLobby, _ctx: &mut Self::Context) -> Self::Result {
        self.assign_lobby(message.0)
    }
}

impl Handler<ReleaseLobby> for RtcPoolActor {
    type Result = ();

    fn handle(&mut self, message: ReleaseLobby, _ctx: &mut Self::Context) {
        if let Some(index) = self.lobby_assignments.remove(&message.0) {
            self.cores[index].lobby_count = self.cores[index].lobby_count.saturating_sub(1);
        }
    }
}

impl Handler<StopRtcPool> for RtcPoolActor {
    type Result = ();

    fn handle(&mut self, _message: StopRtcPool, ctx: &mut Self::Context) {
        ctx.stop();
    }
}

struct RtcCoreLayout {
    bind_ip: IpAddr,
    advertised_ip: IpAddr,
    base_port: u16,
    core_count: usize,
}

impl RtcCoreLayout {
    fn from_config(config: &SfuConfig) -> Result<Self, RtcError> {
        let bind_ip: IpAddr = config
            .bind_ip
            .parse()
            .map_err(|error| RtcError(format!("invalid sfu.bind_ip: {error}")))?;
        let advertised_ip = if config.advertised_ip.is_empty() {
            if bind_ip.is_unspecified() {
                IpAddr::V4(Ipv4Addr::LOCALHOST)
            } else {
                bind_ip
            }
        } else {
            config
                .advertised_ip
                .parse()
                .map_err(|error| RtcError(format!("invalid sfu.advertised_ip: {error}")))?
        };

        let core_count = if config.cores == 0 {
            std::thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(1)
        } else {
            config.cores
        };
        if core_count == 0 {
            return Err(RtcError(
                "RTC core count must be greater than zero".to_owned(),
            ));
        }

        let last_offset = u16::try_from(core_count - 1).map_err(|_| {
            RtcError(format!(
                "sfu.cores={core_count} exceeds the UDP port address space"
            ))
        })?;
        config.base_port.checked_add(last_offset).ok_or_else(|| {
            RtcError(format!(
                "RTC core port block exceeds 65535: base_port={} cores={core_count}",
                config.base_port
            ))
        })?;

        Ok(Self {
            bind_ip,
            advertised_ip,
            base_port: config.base_port,
            core_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consecutive_port_block_is_accepted() {
        let config = SfuConfig {
            cores: 3,
            base_port: 50_000,
            ..Default::default()
        };
        let layout = RtcCoreLayout::from_config(&config).unwrap();
        assert_eq!(layout.core_count, 3);
        assert_eq!(layout.base_port, 50_000);
    }

    #[test]
    fn overflowing_port_block_is_rejected() {
        let config = SfuConfig {
            cores: 3,
            base_port: 65_534,
            ..Default::default()
        };
        assert!(RtcCoreLayout::from_config(&config).is_err());
    }
}
