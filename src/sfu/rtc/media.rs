mod control;
mod demuxer;
mod endpoint;
mod engine;
mod event;
mod forward;
mod lobby;
mod rtcp_forwarder;

pub use engine::MediaEngine;
pub use event::SFUEvent;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RtcDiagnostics {
    pub(crate) packet_io: bool,
    pub(crate) nack_cache: bool,
    pub(crate) forward_timing: bool,
}
