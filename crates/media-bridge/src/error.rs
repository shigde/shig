// Adapted from str0m 0.26.0, src/packet/error.rs. See UPSTREAM.md and LICENSE files.

use std::{error::Error, fmt};

/// Errors encountered while unpacking H.264 RTP payloads.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PacketError {
    /// The payload is too short for its packet format.
    ErrShortPacket,
    /// A STAP-A NAL length exceeds the remaining payload (requested, available).
    StapASizeLargerThanBuffer(usize, usize),
    /// The NAL packet type is unsupported.
    NaluTypeIsNotHandled(u8),
}

impl fmt::Display for PacketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ErrShortPacket => write!(f, "Packet is too short"),
            Self::StapASizeLargerThanBuffer(size, buffer) => {
                write!(f, "H264 StapA size larger than buffer: {size} > {buffer}")
            }
            Self::NaluTypeIsNotHandled(nalu_type) => {
                write!(f, "H264 NALU type is not handled: {nalu_type}")
            }
        }
    }
}

impl Error for PacketError {}
