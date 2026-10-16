use crate::sfu::media::Media;

pub mod actor;
mod error;
pub mod message;
mod pipeline;
mod publisher;

#[cfg(test)]
mod pipeline_test;

#[derive(Clone)]
pub struct RelayMediaStream {
    pub audio: Option<Media>,
    pub video: Option<Media>,
}
