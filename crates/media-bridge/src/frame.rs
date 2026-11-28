// Adapted from str0m 0.26.0, src/packet/buffer_rx.rs. See UPSTREAM.md and LICENSE files.

use std::{num::NonZeroU32, sync::Arc, time::Instant};

use crate::CodecExtra;

/// An RTP sequence number extended beyond the 16-bit wire counter.
///
/// The caller must supply an already extended value. This type does not detect
/// counter wraparound. Values are meaningful within one RTP stream only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SeqNo(pub u64);

/// Media time in extended RTP clock ticks, with the negotiated clock rate.
///
/// This is not local arrival time or a shared audio/video wall clock. The caller
/// must extend the RTP timestamp before constructing this value. Comparing ticks
/// is meaningful only within the same stream and clock domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaTime {
    /// Extended RTP timestamp in clock ticks.
    pub ticks: u64,
    /// Negotiated RTP clock frequency in ticks per second.
    pub clock_rate: NonZeroU32,
}

/// Metadata accompanying one RTP payload supplied to the receive buffer.
#[derive(Debug, Clone)]
pub struct RtpMeta {
    /// Local packet arrival time, used for waiting deadlines.
    pub received: Instant,
    /// Extended media time derived from the packet's RTP timestamp.
    pub time: MediaTime,
    /// Extended sequence number used for ordering and gap detection.
    pub seq_no: SeqNo,
    /// The existing WebRTC stack's header, retaining its RTP extensions.
    pub header: rtp::header::Header,
    /// Latest sender report for this stream, if available.
    ///
    /// Shared rather than copied for every RTP packet. Its raw RTP/NTP mapping
    /// is retained; no audio/video synchronization is performed by this type.
    pub last_sender_info: Option<Arc<rtcp::sender_report::SenderReport>>,
}

/// Compressed output and metadata produced by frame assembly.
///
/// For H.264 this represents an assembled frame; for Opus it represents an RTP
/// payload, which can contain multiple encoded audio frames. This is a data
/// container only: buffering, continuity checks and assembly are separate work.
#[derive(Debug, Clone)]
pub struct Depacketized {
    /// Media time of the assembled output.
    pub time: MediaTime,
    /// Whether this output continues from the preceding output without a gap.
    pub contiguous: bool,
    /// Metadata of contributing RTP packets, in sequence order.
    pub meta: Vec<RtpMeta>,
    /// Compressed media bytes, not decoded pixels or audio samples.
    pub data: Vec<u8>,
    /// Codec-specific metadata collected by the depacketizer.
    pub codec_extra: CodecExtra,
}
