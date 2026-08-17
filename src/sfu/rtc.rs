//! Actix adapter for the Sans-I/O WebRTC media plane.
//!
//! Business actors send coarse-grained signaling and routing commands into this
//! module. UDP packets, protocol timers, RTP forwarding, and RTCP forwarding are
//! driven by [`core_actor::RtcCoreActor`] and do not travel through the lobby/peer mailboxes.

pub mod media_command;
pub mod pool_actor;
pub mod core_actor;
