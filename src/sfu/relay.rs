use crate::sfu::media::Media;

pub mod actor;
mod message;
mod error;
mod port_allocator;
mod cmaf;

#[derive(Clone)]
pub struct RelayMediaStream {
    pub audio: Option<Media>,
    pub video: Option<Media>,
}
