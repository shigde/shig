use crate::sfu::media::Media;

pub mod actor;
mod error;
pub mod message;
pub(crate) mod pipeline;
mod publisher;
mod video_group;
mod video_recovery;

#[cfg(test)]
mod video_recovery_test;

#[cfg(test)]
mod pipeline_test;

#[derive(Clone)]
pub struct RelayMediaStream {
    pub audio: Option<Media>,
    pub video: Option<Media>,
}
