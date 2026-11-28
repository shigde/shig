//! RTP depacketization and encoded frame assembly for Shig.
//!
//! This crate is the destination for the selected str0m packet-processing
//! implementations, adapted to Shig's RTP types without a str0m dependency.
//! H.264 and Opus payload depacketizers are available independently of str0m.
//! Receive buffering and server integration are not implemented yet.

mod error;
mod frame;
mod h264;
mod opus;

pub use error::PacketError;
pub use frame::{Depacketized, MediaTime, RtpMeta, SeqNo};
pub use h264::{H264CodecExtra, H264Depacketizer};
pub use opus::OpusDepacketizer;

/// Codec-specific metadata collected for one assembled output frame.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum CodecExtra {
    /// No codec-specific metadata has been collected yet.
    #[default]
    None,
    /// H.264 metadata.
    H264(H264CodecExtra),
}

/// Processes ordered RTP payloads supplied by a frame-aware receive buffer.
///
/// Callers use a fresh output vector and metadata value for each frame.
/// Implementations may retain fragment state between payloads.
pub trait Depacketizer: std::fmt::Debug {
    /// Estimates output capacity for the combined payload size.
    fn out_size_hint(&self, packets_size: usize) -> Option<usize>;

    /// Appends reconstructed media bytes and updates frame metadata.
    fn depacketize(
        &mut self,
        packet: &[u8],
        out: &mut Vec<u8>,
        codec_extra: &mut CodecExtra,
    ) -> Result<(), PacketError>;

    /// Identifies the start of a codec partition, not necessarily a whole frame.
    fn is_partition_head(&self, packet: &[u8]) -> bool;

    /// Identifies the end of a partition using codec-specific rules.
    fn is_partition_tail(&self, marker: bool, packet: &[u8]) -> bool;
}
