//! Detect unterminated FU-A/FU-B sequences after RTP reordering. GStreamer
//! remains responsible for depayloading and recovery; no media payload is copied.
use gstreamer as gst;

#[derive(Default)]
pub(super) struct H264Guard {
    open: Option<(u32, u32)>, // SSRC, RTP timestamp
    discontinuity: bool,
}

impl H264Guard {
    /// None drops a malformed/orphan fragment; Some(true) resets the depayloader
    /// before this packet so it cannot flush a previous, unfinished NAL as valid.
    pub fn inspect(&mut self, buffer: &gst::BufferRef) -> Option<bool> {
        let mut header = [0u8; 12];
        if buffer.copy_to_slice(0, &mut header).is_err() || header[0] >> 6 != 2 {
            return self.reject();
        }
        let mut offset = 12 + usize::from(header[0] & 15) * 4;
        if header[0] & 0x10 != 0 {
            let mut extension = [0u8; 4];
            if buffer.copy_to_slice(offset, &mut extension).is_err() {
                return self.reject();
            }
            offset += 4 + usize::from(u16::from_be_bytes([extension[2], extension[3]])) * 4;
        }
        let mut payload = [0u8; 2];
        if buffer.copy_to_slice(offset, &mut payload).is_err() {
            return self.reject();
        }
        let identity = (
            u32::from_be_bytes(header[8..12].try_into().unwrap()),
            u32::from_be_bytes(header[4..8].try_into().unwrap()),
        );
        let fragmented = matches!(payload[0] & 31, 28 | 29);
        let start = fragmented && payload[1] & 0x80 != 0;
        let end = fragmented && payload[1] & 0x40 != 0;
        let upstream_discont = buffer.flags().contains(gst::BufferFlags::DISCONT);
        if upstream_discont {
            self.open = None;
        }
        let unfinished = self
            .open
            .is_some_and(|previous| previous != identity || start || !fragmented);
        if unfinished {
            self.open = None;
            self.discontinuity = true;
        }
        if fragmented {
            if start && end {
                return self.reject();
            }
            if !start && self.open != Some(identity) {
                return self.reject();
            }
            self.open = if end { None } else { Some(identity) };
        }
        let reset = std::mem::take(&mut self.discontinuity);
        Some(reset)
    }

    fn reject(&mut self) -> Option<bool> {
        self.open = None;
        self.discontinuity = true;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(timestamp: u32, payload: &[u8]) -> gst::Buffer {
        let mut bytes = vec![0x80, 96, 0, 1];
        bytes.extend_from_slice(&timestamp.to_be_bytes());
        bytes.extend_from_slice(&42u32.to_be_bytes());
        bytes.extend_from_slice(payload);
        gst::Buffer::from_mut_slice(bytes)
    }

    #[test]
    fn complete_fragment_sequence_does_not_reset() {
        gst::init().unwrap();
        let mut guard = H264Guard::default();
        for payload in [&[0x7c, 0x81][..], &[0x7c, 0x01], &[0x7c, 0x41]] {
            assert_eq!(guard.inspect(&packet(10, payload)), Some(false));
        }
        assert_eq!(guard.inspect(&packet(20, &[0x61, 0])), Some(false));
    }

    #[test]
    fn missing_end_resets_before_aggregation_or_new_fragment_start() {
        gst::init().unwrap();
        for next in [&[0x78, 0][..], &[0x7c, 0x85]] {
            let mut guard = H264Guard::default();
            assert_eq!(guard.inspect(&packet(10, &[0x7c, 0x81])), Some(false));
            assert_eq!(guard.inspect(&packet(10, &[0x7c, 0x01])), Some(false));
            assert_eq!(guard.inspect(&packet(20, next)), Some(true));
        }
    }

    #[test]
    fn orphan_continuation_is_dropped_until_a_new_start() {
        gst::init().unwrap();
        let mut guard = H264Guard::default();
        assert_eq!(guard.inspect(&packet(10, &[0x7c, 0x01])), None);
        assert_eq!(guard.inspect(&packet(10, &[0x7c, 0x41])), None);
        assert_eq!(guard.inspect(&packet(20, &[0x7c, 0x85])), Some(true));
        assert_eq!(guard.inspect(&packet(20, &[0x7c, 0x45])), Some(false));
    }
}
