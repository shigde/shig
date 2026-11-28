// Adapted from str0m 0.26.0, src/packet/h264.rs. See UPSTREAM.md and LICENSE files.

use crate::{CodecExtra, Depacketizer, PacketError};

/// Metadata collected while assembling an H.264 frame.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct H264CodecExtra {
    /// Whether the assembled output contains an IDR NAL unit.
    pub is_keyframe: bool,
}

const STAPA_NALU_TYPE: u8 = 24;
const FUA_NALU_TYPE: u8 = 28;
const FUB_NALU_TYPE: u8 = 29;
const IDR_NALU_TYPE: u8 = 5;
const FUA_HEADER_SIZE: usize = 2;
const STAPA_HEADER_SIZE: usize = 1;
const STAPA_NALU_LENGTH_SIZE: usize = 2;
const NALU_TYPE_BITMASK: u8 = 0x1F;
const NALU_REF_IDC_BITMASK: u8 = 0x60;
const FU_START_BITMASK: u8 = 0x80;
const FU_END_BITMASK: u8 = 0x40;
const ANNEXB_NALUSTART_CODE: &[u8] = &[0x00, 0x00, 0x00, 0x01];

/// Depacketizes H264 RTP packets.
///
/// Payloads must be supplied in order by a frame-aware receive buffer.
/// This type does not reorder packets, detect sequence gaps, or request retransmissions.
#[derive(PartialEq, Eq, Debug, Default, Clone)]
pub struct H264Depacketizer {
    /// Whether to output in AVC format (length-prefixed NALUs) instead of Annex B format
    pub is_avc: bool,
    fua_buffer: Option<Vec<u8>>,
}

impl Depacketizer for H264Depacketizer {
    fn out_size_hint(&self, packets_size: usize) -> Option<usize> {
        // Roughly account for Annex B start codes or AVC length prefixes.
        let estimated_packets = (packets_size / 1200).saturating_add(1);
        Some(packets_size.saturating_add(4usize.saturating_mul(estimated_packets)))
    }

    /// depacketize parses the passed byte slice and stores the result in the
    /// H264Packet this method is called upon
    fn depacketize(
        &mut self,
        packet: &[u8],
        out: &mut Vec<u8>,
        extra: &mut CodecExtra,
    ) -> Result<(), PacketError> {
        if packet.len() == 0 {
            return Err(PacketError::ErrShortPacket);
        }

        // NALU Types
        // https://tools.ietf.org/html/rfc6184#section-5.4
        let b0 = packet[0];
        let nalu_type = b0 & NALU_TYPE_BITMASK;

        match nalu_type {
            t @ 1..=23 => {
                let is_keyframe = if let CodecExtra::H264(e) = extra {
                    (t == IDR_NALU_TYPE) | e.is_keyframe
                } else {
                    t == IDR_NALU_TYPE
                };
                *extra = CodecExtra::H264(H264CodecExtra { is_keyframe });

                if self.is_avc {
                    out.extend_from_slice(&(packet.len() as u32).to_be_bytes());
                } else {
                    out.extend_from_slice(ANNEXB_NALUSTART_CODE);
                }
                out.extend_from_slice(packet);
                Ok(())
            }
            STAPA_NALU_TYPE => {
                let mut curr_offset = STAPA_HEADER_SIZE;
                while curr_offset + 1 < packet.len() {
                    let nalu_size =
                        ((packet[curr_offset] as usize) << 8) | packet[curr_offset + 1] as usize;
                    curr_offset += STAPA_NALU_LENGTH_SIZE;

                    if curr_offset + nalu_size > packet.len() {
                        return Err(PacketError::StapASizeLargerThanBuffer(
                            nalu_size,
                            packet.len() - curr_offset,
                        ));
                    }

                    let Some(b0) = packet.get(curr_offset) else {
                        continue;
                    };
                    let t = b0 & NALU_TYPE_BITMASK;
                    let is_keyframe = if let CodecExtra::H264(e) = extra {
                        (t == IDR_NALU_TYPE) | e.is_keyframe
                    } else {
                        t == IDR_NALU_TYPE
                    };
                    *extra = CodecExtra::H264(H264CodecExtra { is_keyframe });

                    if self.is_avc {
                        out.extend_from_slice(&(nalu_size as u32).to_be_bytes());
                    } else {
                        out.extend_from_slice(ANNEXB_NALUSTART_CODE);
                    }
                    out.extend_from_slice(&packet[curr_offset..curr_offset + nalu_size]);
                    curr_offset += nalu_size;
                }

                Ok(())
            }
            FUA_NALU_TYPE => {
                if packet.len() < FUA_HEADER_SIZE as usize {
                    return Err(PacketError::ErrShortPacket);
                }

                if self.fua_buffer.is_none() {
                    self.fua_buffer = Some(Vec::new());
                }

                if let Some(fua_buffer) = &mut self.fua_buffer {
                    fua_buffer.extend_from_slice(&packet[FUA_HEADER_SIZE as usize..]);
                }

                let b1 = packet[1];
                if b1 & FU_END_BITMASK != 0 {
                    let nalu_ref_idc = b0 & NALU_REF_IDC_BITMASK;
                    let fragmented_nalu_type = b1 & NALU_TYPE_BITMASK;

                    let is_keyframe = if let CodecExtra::H264(e) = extra {
                        (fragmented_nalu_type == IDR_NALU_TYPE) | e.is_keyframe
                    } else {
                        fragmented_nalu_type == IDR_NALU_TYPE
                    };
                    *extra = CodecExtra::H264(H264CodecExtra { is_keyframe });

                    if let Some(fua_buffer) = self.fua_buffer.take() {
                        if self.is_avc {
                            out.extend_from_slice(&((fua_buffer.len() + 1) as u32).to_be_bytes());
                        } else {
                            out.extend_from_slice(ANNEXB_NALUSTART_CODE);
                        }
                        out.push(nalu_ref_idc | fragmented_nalu_type);
                        out.extend_from_slice(&fua_buffer);
                    }

                    Ok(())
                } else {
                    Ok(())
                }
            }
            _ => Err(PacketError::NaluTypeIsNotHandled(nalu_type)),
        }
    }

    /// is_partition_head checks if this is the head of a packetized nalu stream.
    fn is_partition_head(&self, packet: &[u8]) -> bool {
        if packet.len() < 2 {
            return false;
        }

        if packet[0] & NALU_TYPE_BITMASK == FUA_NALU_TYPE
            || packet[0] & NALU_TYPE_BITMASK == FUB_NALU_TYPE
        {
            (packet[1] & FU_START_BITMASK) != 0
        } else {
            true
        }
    }

    fn is_partition_tail(&self, marker: bool, _packet: &[u8]) -> bool {
        marker
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_h264 {
        ($name:tt, $is_avc:expr, $is_ok: expr, $payload:expr, $err:tt) => {
            #[test]
            fn $name() -> Result<(), PacketError> {
                let mut pkt = H264Depacketizer::default();
                pkt.is_avc = $is_avc;
                let mut extra = CodecExtra::None;
                let mut out: Vec<u8> = Vec::new();
                let result = pkt.depacketize($payload, &mut out, &mut extra);
                if $is_ok {
                    assert!(result.is_ok(), $err);
                } else {
                    assert!(result.is_err(), $err);
                }
                Ok(())
            }
        };
    }

    test_h264!(
        nil_payload,
        false,
        false,
        &[],
        "Unmarshal did not fail on nil payload"
    );
    test_h264!(
        unit_delimiter,
        false,
        true,
        &[0x09, 0x30],
        "Unmarshal should accept minimal h.264 access unit delimiter"
    );
    test_h264!(
        end_of_sequence_nalu,
        false,
        true,
        &[0x0A],
        "Unmarshal should accept end of sequence NALU"
    );
    test_h264!(
        not_handled,
        false,
        false,
        &[0xFF, 0x00, 0x00],
        "Unmarshal accepted a packet with a NALU Type we don't handle"
    );
    test_h264!(
        incomplete_single_payload_multi_nalu,
        false,
        false,
        &[
            0x78, 0x00, 0x0f, 0x67, 0x42, 0xc0, 0x1f, 0x1a, 0x32, 0x35, 0x01, 0x40, 0x7a, 0x40,
            0x3c, 0x22, 0x11,
        ],
        "Unmarshal accepted a STAP-A packet with insufficient data"
    );

    #[test]
    fn single_payload() -> Result<(), PacketError> {
        let mut pkt = H264Depacketizer::default();
        let mut extra = CodecExtra::None;
        let mut out: Vec<u8> = Vec::new();
        let single_payload = &[0x90, 0x90, 0x90];
        let _ = pkt.depacketize(single_payload, &mut out, &mut extra);
        let single_payload_unmarshaled = &[0x00, 0x00, 0x00, 0x01, 0x90, 0x90, 0x90];
        assert_eq!(
            out, single_payload_unmarshaled,
            "Unmarshaling a single payload shouldn't modify the payload"
        );
        Ok(())
    }

    #[test]
    fn single_payload_avc() -> Result<(), PacketError> {
        let mut pkt = H264Depacketizer::default();
        pkt.is_avc = true;
        let mut extra = CodecExtra::None;
        let mut out: Vec<u8> = Vec::new();
        let single_payload = &[0x90, 0x90, 0x90];
        let _ = pkt.depacketize(single_payload, &mut out, &mut extra);
        let single_payload_unmarshaled_avc = &[0x00, 0x00, 0x00, 0x03, 0x90, 0x90, 0x90];
        assert_eq!(
            out, single_payload_unmarshaled_avc,
            "Unmarshaling a single payload into avc stream shouldn't modify the payload"
        );
        Ok(())
    }

    #[test]
    fn h264_large_out() -> Result<(), PacketError> {
        let large_payload_packetized = vec![
            &[0x1c, 0x80, 0x01, 0x02, 0x03],
            &[0x1c, 0x00, 0x04, 0x05, 0x06],
            &[0x1c, 0x00, 0x07, 0x08, 0x09],
            &[0x1c, 0x00, 0x10, 0x11, 0x12],
            &[0x1c, 0x40, 0x13, 0x14, 0x15],
        ];

        let large_payload = &[
            0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09,
            0x10, 0x11, 0x12, 0x13, 0x14, 0x15,
        ];

        let mut pkt = H264Depacketizer::default();
        let mut extra = CodecExtra::None;

        let mut large_out = Vec::new();
        for p in &large_payload_packetized {
            pkt.depacketize(*p, &mut large_out, &mut extra)?;
        }
        assert_eq!(
            large_out, large_payload,
            "Failed to unmarshal a large payload"
        );

        Ok(())
    }

    #[test]
    fn h264_large_out_avc() -> Result<(), PacketError> {
        let large_payload_packetized = vec![
            &[0x1c, 0x80, 0x01, 0x02, 0x03],
            &[0x1c, 0x00, 0x04, 0x05, 0x06],
            &[0x1c, 0x00, 0x07, 0x08, 0x09],
            &[0x1c, 0x00, 0x10, 0x11, 0x12],
            &[0x1c, 0x40, 0x13, 0x14, 0x15],
        ];

        let large_payload_avc = &[
            0x00, 0x00, 0x00, 0x10, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09,
            0x10, 0x11, 0x12, 0x13, 0x14, 0x15,
        ];

        let mut avc_pkt = H264Depacketizer {
            is_avc: true,
            ..Default::default()
        };

        let mut extra = CodecExtra::None;

        let mut large_out_avc = Vec::new();
        for p in &large_payload_packetized {
            avc_pkt.depacketize(*p, &mut large_out_avc, &mut extra)?;
        }
        assert_eq!(
            large_out_avc, large_payload_avc,
            "Failed to unmarshal a large payload into avc stream"
        );

        Ok(())
    }

    #[test]
    fn single_payload_multi_nalu() -> Result<(), PacketError> {
        let single_payload_multi_nalu = &[
            0x78, 0x00, 0x0f, 0x67, 0x42, 0xc0, 0x1f, 0x1a, 0x32, 0x35, 0x01, 0x40, 0x7a, 0x40,
            0x3c, 0x22, 0x11, 0xa8, 0x00, 0x05, 0x68, 0x1a, 0x34, 0xe3, 0xc8, 0x00,
        ];
        let single_payload_multi_nalu_unmarshaled = &[
            0x00, 0x00, 0x00, 0x01, 0x67, 0x42, 0xc0, 0x1f, 0x1a, 0x32, 0x35, 0x01, 0x40, 0x7a,
            0x40, 0x3c, 0x22, 0x11, 0xa8, 0x00, 0x00, 0x00, 0x01, 0x68, 0x1a, 0x34, 0xe3, 0xc8,
        ];

        let mut pkt = H264Depacketizer::default();

        let mut extra = CodecExtra::None;

        let mut out = Vec::new();
        pkt.depacketize(single_payload_multi_nalu, &mut out, &mut extra)?;
        assert_eq!(
            out, single_payload_multi_nalu_unmarshaled,
            "Failed to unmarshal a single packet with multiple NALUs"
        );

        Ok(())
    }

    #[test]
    fn single_payload_multi_nalu_avc() -> Result<(), PacketError> {
        let single_payload_multi_nalu = &[
            0x78, 0x00, 0x0f, 0x67, 0x42, 0xc0, 0x1f, 0x1a, 0x32, 0x35, 0x01, 0x40, 0x7a, 0x40,
            0x3c, 0x22, 0x11, 0xa8, 0x00, 0x05, 0x68, 0x1a, 0x34, 0xe3, 0xc8, 0x00,
        ];
        let single_payload_multi_nalu_unmarshaled_avc = &[
            0x00, 0x00, 0x00, 0x0f, 0x67, 0x42, 0xc0, 0x1f, 0x1a, 0x32, 0x35, 0x01, 0x40, 0x7a,
            0x40, 0x3c, 0x22, 0x11, 0xa8, 0x00, 0x00, 0x00, 0x05, 0x68, 0x1a, 0x34, 0xe3, 0xc8,
        ];

        let mut avc_pkt = H264Depacketizer::default();
        avc_pkt.is_avc = true;

        let mut extra = CodecExtra::None;

        let mut out = Vec::new();
        avc_pkt.depacketize(single_payload_multi_nalu, &mut out, &mut extra)?;
        assert_eq!(
            out, single_payload_multi_nalu_unmarshaled_avc,
            "Failed to unmarshal a single packet with multiple NALUs into avc stream"
        );

        Ok(())
    }

    #[test]
    fn test_h264_partition_head_checker_is_partition_head() -> Result<(), PacketError> {
        let h264 = H264Depacketizer::default();
        let empty_nalu = &[];
        assert!(
            !h264.is_partition_head(empty_nalu),
            "empty nalu must not be a partition head"
        );

        let single_nalu = &[1, 0];
        assert!(
            h264.is_partition_head(single_nalu),
            "single nalu must be a partition head"
        );

        let stapa_nalu = &[STAPA_NALU_TYPE, 0];
        assert!(
            h264.is_partition_head(stapa_nalu),
            "stapa nalu must be a partition head"
        );

        let fua_start_nalu = &[FUA_NALU_TYPE, FU_START_BITMASK];
        assert!(
            h264.is_partition_head(fua_start_nalu),
            "fua start nalu must be a partition head"
        );

        let fua_end_nalu = &[FUA_NALU_TYPE, FU_END_BITMASK];
        assert!(
            !h264.is_partition_head(fua_end_nalu),
            "fua end nalu must not be a partition head"
        );

        let fub_start_nalu = &[FUB_NALU_TYPE, FU_START_BITMASK];
        assert!(
            h264.is_partition_head(fub_start_nalu),
            "fub start nalu must be a partition head"
        );

        let fub_end_nalu = &[FUB_NALU_TYPE, FU_END_BITMASK];
        assert!(
            !h264.is_partition_head(fub_end_nalu),
            "fub end nalu must not be a partition head"
        );

        Ok(())
    }

    #[test]
    fn test_h264_depacketizer_idr_handling() -> Result<(), PacketError> {
        let mut pck = H264Depacketizer::default();
        let mut extra = CodecExtra::None;
        let mut out = vec![];

        // First byte is NALU type
        let packet = [0x85];
        pck.depacketize(&packet, &mut out, &mut extra)?;
        let CodecExtra::H264(e) = extra else {
            panic!("Expected CodecExtra::H264");
        };
        assert!(e.is_keyframe);

        // First byte is STAPA NALU type
        let packet = vec![
            vec![
                120, 0, 15, 103, 66, 192, 21, 140, 141, 64, 160, 203, 207, 0, 240, 136, 70, 160, 0,
                4, 104, 206, 60, 128, 1, 20, 101,
            ],
            vec![0; 276],
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        pck.depacketize(packet.as_slice(), &mut out, &mut extra)?;
        let CodecExtra::H264(e) = extra else {
            panic!("Expected CodecExtra::H264");
        };
        assert!(e.is_keyframe);

        // First byte is FUA NALU type
        let packet = [124, 69];
        pck.depacketize(&packet, &mut out, &mut extra)?;
        let CodecExtra::H264(e) = extra else {
            panic!("Expected CodecExtra::H264");
        };
        assert!(e.is_keyframe);
        Ok(())
    }

    #[test]
    fn parse_first_packet() {
        const PACKET: &[u8] = &[
            120, 000, 015, 103, 066, 192, 021, 140, 141, 064, 160, 203, 207, 000, 240, 136, 070,
            160, 000, 004, 104, 206, 060, 128, 000, 204, 101, 184, 000, 004, 000, 000, 005, 057,
            049, 064, 000, 064, 222, 078, 078, 078, 078, 078, 078, 078, 078, 078, 078, 078, 078,
            078, 078, 078, 078, 078, 078, 078, 186, 235, 174, 186, 235, 174, 186, 235, 174, 186,
            235, 174, 186, 235, 174, 186, 235, 174, 186, 235, 174, 186, 235, 174, 186, 235, 174,
            186, 235, 174, 186, 235, 174, 186, 235, 174, 186, 235, 173, 223, 039, 125, 247, 223,
            125, 245, 215, 093, 117, 215, 093, 117, 214, 239, 174, 187, 235, 174, 186, 235, 174,
            186, 235, 174, 186, 235, 174, 186, 235, 174, 186, 235, 174, 186, 235, 174, 186, 235,
            174, 186, 235, 174, 183, 093, 117, 215, 093, 117, 215, 093, 117, 215, 093, 117, 215,
            093, 117, 215, 093, 117, 215, 092, 189, 117, 215, 093, 117, 215, 093, 117, 215, 093,
            117, 215, 093, 117, 215, 093, 117, 215, 093, 117, 214, 239, 190, 251, 239, 190, 186,
            235, 174, 186, 235, 174, 186, 235, 174, 186, 235, 174, 186, 235, 174, 186, 235, 174,
            186, 235, 174, 186, 235, 175, 227, 255, 240, 247, 021, 223, 125, 247, 223, 125, 247,
            223, 125, 247, 223, 125, 247, 223, 125, 248,
        ];

        let mut pck = H264Depacketizer::default();
        let mut extra = CodecExtra::None;
        let mut out = vec![];
        pck.depacketize(PACKET, &mut out, &mut extra).unwrap();
    }

    #[test]
    fn test_out_of_bounds_access() {
        const PACKET: &[u8] = &[STAPA_NALU_TYPE, 0x00, 0x00];

        let mut pck = H264Depacketizer::default();
        let mut extra = CodecExtra::None;
        let mut out = vec![];
        pck.depacketize(PACKET, &mut out, &mut extra).unwrap();
    }
}
