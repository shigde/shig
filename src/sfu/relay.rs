use crate::sfu::media::Media;

pub mod actor;
pub mod message;
mod error;
mod port_allocator;
mod cmaf;
mod rtp_forwarder;
mod actor_supervisor;

#[derive(Clone)]
pub struct RelayMediaStream {
    pub audio: Option<Media>,
    pub video: Option<Media>,
}
