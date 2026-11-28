// Adapted from str0m 0.26.0, src/packet/opus.rs. See UPSTREAM.md and LICENSE files.

use crate::{CodecExtra, Depacketizer, PacketError};

/// Extracts Opus payload bytes without decoding or validating the Opus bitstream.
///
/// Each RTP payload is treated as a complete partition, independently of the
/// marker bit. An Opus packet may itself contain multiple encoded audio frames.
#[derive(PartialEq, Eq, Debug, Default, Clone)]
pub struct OpusDepacketizer;

impl Depacketizer for OpusDepacketizer {
    fn out_size_hint(&self, packets_size: usize) -> Option<usize> {
        Some(packets_size)
    }

    fn depacketize(
        &mut self,
        packet: &[u8],
        out: &mut Vec<u8>,
        _: &mut CodecExtra,
    ) -> Result<(), PacketError> {
        if !packet.is_empty() {
            out.extend_from_slice(packet);
        }

        Ok(())
    }

    fn is_partition_head(&self, _payload: &[u8]) -> bool {
        true
    }

    fn is_partition_tail(&self, _marker: bool, _payload: &[u8]) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opus_unmarshal() -> Result<(), PacketError> {
        let mut pck = OpusDepacketizer;
        let mut extra = CodecExtra::None;

        // Empty packet
        let empty_bytes = &[];
        let mut out = Vec::new();
        let result = pck.depacketize(empty_bytes, &mut out, &mut extra);
        assert!(
            result.is_ok(),
            "Result should not be err in case of empty packet"
        );

        // Normal packet
        let raw_bytes: &[u8] = &[0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x90];
        let mut out = Vec::new();
        pck.depacketize(raw_bytes, &mut out, &mut extra)?;
        assert_eq!(raw_bytes, &out, "Payload must be same");

        Ok(())
    }

    #[test]
    fn test_opus_is_partition_head() -> Result<(), PacketError> {
        let opus = OpusDepacketizer;
        assert!(
            opus.is_partition_head(&[0x00, 0x00]),
            "All OPUS RTP packet should be the head of a new partition"
        );

        Ok(())
    }
}
