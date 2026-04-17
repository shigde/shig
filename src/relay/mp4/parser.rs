// Full MP4 box parsing for CMAF-IF ingest: codec detection, trun rewriting,
// init segment parsing, moof+mdat range scanning.

use anyhow::{anyhow, Result};

#[derive(Debug, Clone)]
pub struct BoxRange {
    pub start: u64,
    pub end: u64,
}

pub fn parse_moof_mdat_ranges(data: &[u8]) -> Result<Vec<BoxRange>> {
    let mut ranges = Vec::new();
    let mut offset = 0u64;
    let mut pending_moof_start: Option<u64> = None;

    while offset + 8 <= data.len() as u64 {
        let size32 = read_u32_be(data, offset)?;
        let box_type = read_box_type(data, offset + 4)?;

        let (box_size, header_size) = match size32 {
            0 => ((data.len() as u64) - offset, 8u64),
            1 => (read_u64_be(data, offset + 8)?, 16u64),
            _ => (size32 as u64, 8u64),
        };

        if box_size < header_size {
            break;
        }

        let box_start = offset;
        let box_end = offset + box_size - 1;

        match box_type.as_str() {
            "moof" => {
                if let Some(start) = pending_moof_start {
                    if start < box_start {
                        ranges.push(BoxRange { start, end: box_start - 1 });
                    }
                }
                pending_moof_start = Some(box_start);
            }
            "mdat" => {
                if let Some(start) = pending_moof_start {
                    ranges.push(BoxRange { start, end: box_end });
                    pending_moof_start = None;
                }
            }
            _ => {}
        }

        offset += box_size;
    }

    if let Some(start) = pending_moof_start {
        ranges.push(BoxRange { start, end: (data.len() - 1) as u64 });
    }

    Ok(ranges)
}

fn read_u32_be(data: &[u8], offset: u64) -> Result<u32> {
    let offset = offset as usize;
    if offset + 4 > data.len() {
        return Err(anyhow!("Read out of bounds"));
    }
    Ok(u32::from_be_bytes([
        data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
    ]))
}

fn read_u64_be(data: &[u8], offset: u64) -> Result<u64> {
    let offset = offset as usize;
    if offset + 8 > data.len() {
        return Err(anyhow!("Read out of bounds"));
    }
    Ok(u64::from_be_bytes([
        data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
        data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7],
    ]))
}

fn read_box_type(data: &[u8], offset: u64) -> Result<String> {
    let offset = offset as usize;
    if offset + 4 > data.len() {
        return Err(anyhow!("Read out of bounds"));
    }
    Ok(String::from_utf8_lossy(&data[offset..offset + 4]).to_string())
}

pub fn parse_base_decode_time(data: &[u8]) -> Option<u64> {
    let moof_content = find_box_content(data, b"moof")?;
    let traf_content = find_box_content(moof_content, b"traf")?;
    let tfdt_content = find_box_content(traf_content, b"tfdt")?;

    if tfdt_content.len() < 8 { return None; }
    let version = tfdt_content[0];
    if version == 1 {
        if tfdt_content.len() < 12 { return None; }
        Some(u64::from_be_bytes([
            tfdt_content[4], tfdt_content[5], tfdt_content[6], tfdt_content[7],
            tfdt_content[8], tfdt_content[9], tfdt_content[10], tfdt_content[11],
        ]))
    } else {
        Some(u32::from_be_bytes([
            tfdt_content[4], tfdt_content[5], tfdt_content[6], tfdt_content[7],
        ]) as u64)
    }
}

/// Returns whether the first sample in this fragment is a sync sample (IDR/keyframe).
///
/// Returns `Some(true)` if sync, `Some(false)` if non-sync, `None` if no
/// sample flags were found in the fragment (caller decides policy).
///
/// Checks (in priority order):
/// 1. trun first_sample_flags (tr_flags & 0x004)
/// 2. trun per-sample flags for sample 0 (tr_flags & 0x400)
/// 3. tfhd default_sample_flags (tf_flags & 0x020000)
///
/// A sample is sync when bit 16 (sample_is_non_sync_sample) is clear.
pub fn fragment_starts_with_idr(data: &[u8]) -> Option<bool> {
    let moof = match find_box_content(data, b"moof") {
        Some(c) => c,
        None => return None,
    };
    let traf = match find_box_content(moof, b"traf") {
        Some(c) => c,
        None => return None,
    };

    // Parse tfhd default_sample_flags (fallback)
    let mut default_flags: Option<u32> = None;
    if let Some(tfhd) = find_box_content(traf, b"tfhd") {
        if tfhd.len() >= 4 {
            let tf_flags = u32::from_be_bytes([0, tfhd[1], tfhd[2], tfhd[3]]);
            // default-sample-flags-present is bit 5 (0x020000) — but bit numbering:
            // tf_flags & 0x000020 means default-sample-flags-present
            if tf_flags & 0x000020 != 0 {
                // Skip: track_id (4 bytes after version+flags)
                let mut off = 4 + 4; // version+flags + track_id
                if tf_flags & 0x000001 != 0 { off += 8; } // base-data-offset
                if tf_flags & 0x000002 != 0 { off += 4; } // sample-description-index
                if tf_flags & 0x000008 != 0 { off += 4; } // default-sample-duration
                if tf_flags & 0x000010 != 0 { off += 4; } // default-sample-size
                if off + 4 <= tfhd.len() {
                    default_flags = Some(u32::from_be_bytes([
                        tfhd[off], tfhd[off+1], tfhd[off+2], tfhd[off+3],
                    ]));
                }
            }
        }
    }

    // Parse trun to get first sample flags
    if let Some(trun) = find_box_content(traf, b"trun") {
        if trun.len() < 8 { return default_flags.map(|f| f & 0x10000 == 0); }
        let tr_flags = u32::from_be_bytes([0, trun[1], trun[2], trun[3]]);

        // Priority 1: first_sample_flags (tr_flags & 0x004)
        if tr_flags & 0x004 != 0 {
            let mut off = 4 + 4; // version+flags + sample_count
            if tr_flags & 0x001 != 0 { off += 4; } // data_offset
            if off + 4 <= trun.len() {
                let first_flags = u32::from_be_bytes([
                    trun[off], trun[off+1], trun[off+2], trun[off+3],
                ]);
                let is_sync = first_flags & 0x10000 == 0;
                log::debug!("IDR detect: trun first_sample_flags=0x{:08x} default_flags={:?} sync={}",
                    first_flags, default_flags, is_sync);

                // Trust first_sample_flags=non-sync unconditionally (encoder won't
                // mark an IDR as non-sync).
                if !is_sync {
                    return Some(false);
                }

                // first_sample_flags=sync: verify with NAL scan. Some encoders
                // (Ateme Titan) set first_sample_flags=0x02000000 on EVERY chunk
                // regardless of IDR. NAL scanning is the ground truth.
                if let Some(nal) = mdat_contains_idr(data) {
                    log::debug!("IDR detect: first_sample_flags=sync, NAL verify={}", nal);
                    return Some(nal);
                }
                // NAL scan inconclusive — trust the flags
                return Some(true);
            }
        }

        // Priority 2: per-sample flags for sample 0 (tr_flags & 0x400)
        if tr_flags & 0x400 != 0 {
            let mut off = 4 + 4; // version+flags + sample_count
            if tr_flags & 0x001 != 0 { off += 4; } // data_offset
            // (no first_sample_flags since we checked above)
            // Per-sample record: duration(4?) + size(4?) + flags(4) + cts(4?)
            if tr_flags & 0x100 != 0 { off += 4; } // sample_duration
            if tr_flags & 0x200 != 0 { off += 4; } // sample_size
            if off + 4 <= trun.len() {
                let sample_flags = u32::from_be_bytes([
                    trun[off], trun[off+1], trun[off+2], trun[off+3],
                ]);
                let is_sync = sample_flags & 0x10000 == 0;
                log::debug!("IDR detect: trun per-sample flags=0x{:08x} sync={}", sample_flags, is_sync);
                return Some(is_sync);
            }
        }

        log::debug!("IDR detect: trun has no sample flags, tr_flags=0x{:06x}", tr_flags);
    }

    // Fallback: tfhd default_sample_flags
    if let Some(f) = default_flags {
        let is_sync = f & 0x10000 == 0;
        log::debug!("IDR detect: tfhd default_sample_flags=0x{:08x} sync={}", f, is_sync);
        return Some(is_sync);
    }

    log::debug!("IDR detect: no flags found, falling back to NAL scan");
    // Last resort: scan mdat for H.264/H.265 NAL units to detect IDR frames.
    // CMAF uses length-prefixed NALs (not Annex B start codes).
    let nal_result = mdat_contains_idr(data);
    log::debug!("IDR detect: NAL scan result={:?}", nal_result);
    nal_result
}

/// Scan the mdat box for H.264 IDR NAL units (type 5) or H.265 IRAP NAL units (types 16-21).
/// CMAF uses ISO BMFF length-prefixed NAL format (4-byte big-endian length + NAL data).
fn mdat_contains_idr(data: &[u8]) -> Option<bool> {
    // Find the mdat box, tolerating truncated data (HTTP chunked encoding
    // may deliver partial mdat). We only need the first few NAL headers.
    let mdat = find_mdat_content_partial(data)?;
    if mdat.is_empty() {
        return None;
    }

    log::debug!("NAL scan: mdat len={} first_bytes={:02x?}",
        mdat.len(), &mdat[..mdat.len().min(16)]);

    let mut off = 0;
    let mut nal_idx = 0;
    while off + 4 <= mdat.len() {
        let nal_len = u32::from_be_bytes([mdat[off], mdat[off+1], mdat[off+2], mdat[off+3]]) as usize;
        off += 4;
        if nal_len == 0 {
            break;
        }
        if off + nal_len > mdat.len() {
            // NAL body is truncated (chunked HTTP delivery), but the header
            // byte at mdat[off] is still readable — peek at it.
            if off < mdat.len() {
                let nal_header = mdat[off];
                let h264_type = nal_header & 0x1F;
                log::debug!("NAL scan: nal[{}] truncated (len={} avail={}) header=0x{:02x} h264_type={}",
                    nal_idx, nal_len, mdat.len() - off, nal_header, h264_type);
                if h264_type == 5 { return Some(true); }
                if h264_type == 1 { return Some(false); }
            }
            break;
        }

        let nal_header = mdat[off];
        let h264_type = nal_header & 0x1F;
        log::debug!("NAL scan: nal[{}] header=0x{:02x} h264_type={} len={}", nal_idx, nal_header, h264_type, nal_len);

        // H.264 coded slices — definitive answer
        if h264_type == 5 { return Some(true); }   // IDR
        if h264_type == 1 { return Some(false); }   // non-IDR slice

        // H.264 non-VCL types (SEI=6, SPS=7, PPS=8, AUD=9, etc.) — skip.
        if (2..=12).contains(&h264_type) {
            off += nal_len;
            nal_idx += 1;
            continue;
        }

        // Not a recognized H.264 type (0 or 13+) — try HEVC interpretation.
        if off + 1 < mdat.len() {
            let hevc_type = (nal_header >> 1) & 0x3F;
            if (16..=21).contains(&hevc_type) { return Some(true); }   // IRAP
            if hevc_type <= 9 { return Some(false); }                   // non-IRAP VCL
        }

        off += nal_len;
        nal_idx += 1;
    }

    log::debug!("NAL scan: no VCL NAL found after {} NALs", nal_idx);
    None
}

pub fn parse_timescale(data: &[u8]) -> Option<u32> {
    let moov_content = find_box_content(data, b"moov")?;
    let trak_content = find_box_content(moov_content, b"trak")?;
    let mdia_content = find_box_content(trak_content, b"mdia")?;
    let mdhd_content = find_box_content(mdia_content, b"mdhd")?;

    let version = mdhd_content[0];
    if version == 1 {
        if mdhd_content.len() < 24 { return None; }
        Some(u32::from_be_bytes([
            mdhd_content[20], mdhd_content[21], mdhd_content[22], mdhd_content[23],
        ]))
    } else {
        if mdhd_content.len() < 16 { return None; }
        Some(u32::from_be_bytes([
            mdhd_content[12], mdhd_content[13], mdhd_content[14], mdhd_content[15],
        ]))
    }
}

pub fn parse_codec_from_init(data: &[u8]) -> Option<String> {
    let moov = find_box_content(data, b"moov")?;
    let trak = find_box_content(moov, b"trak")?;
    let mdia = find_box_content(trak, b"mdia")?;
    let minf = find_box_content(mdia, b"minf")?;
    let stbl = find_box_content(minf, b"stbl")?;
    let stsd = find_box_content(stbl, b"stsd")?;

    if stsd.len() < 8 { return None; }
    let entry_data = &stsd[8..];

    if let Some(avc1_content) = find_box_content(entry_data, b"avc1") {
        if avc1_content.len() > 78 {
            if let Some(avcc_content) = find_box_content(&avc1_content[78..], b"avcC") {
                if avcc_content.len() >= 4 {
                    return Some(format!(
                        "avc1.{:02x}{:02x}{:02x}",
                        avcc_content[1], avcc_content[2], avcc_content[3]
                    ));
                }
            }
        }
    }

    if let Some(mp4a_content) = find_box_content(entry_data, b"mp4a") {
        if mp4a_content.len() > 28 {
            if let Some(esds_content) = find_box_content(&mp4a_content[28..], b"esds") {
                return parse_esds_codec(esds_content);
            }
        }
    }

    None
}

fn parse_esds_codec(data: &[u8]) -> Option<String> {
    if data.len() < 4 { return None; }
    let data = &data[4..];

    let mut i = 0;
    if i >= data.len() || data[i] != 0x03 { return None; }
    i = skip_descriptor_length(data, i + 1)?;
    i += 3;

    if i >= data.len() || data[i] != 0x04 { return None; }
    i = skip_descriptor_length(data, i + 1)?;
    if i >= data.len() { return None; }
    let object_type = data[i];
    i += 13;

    if i >= data.len() || data[i] != 0x05 {
        return Some(format!("mp4a.{:02x}.2", object_type));
    }
    i = skip_descriptor_length(data, i + 1)?;

    if i >= data.len() {
        return Some(format!("mp4a.{:02x}.2", object_type));
    }
    let mut audio_object_type = (data[i] >> 3) as u32;
    if audio_object_type == 31 && i + 1 < data.len() {
        audio_object_type = 32 + (((data[i] & 0x07) as u32) << 3) | ((data[i + 1] >> 5) as u32);
    }
    Some(format!("mp4a.{:02x}.{}", object_type, audio_object_type))
}

fn skip_descriptor_length(data: &[u8], offset: usize) -> Option<usize> {
    let mut i = offset;
    while i < data.len() && (data[i] & 0x80) != 0 {
        i += 1;
    }
    if i < data.len() { Some(i + 1) } else { None }
}

pub fn has_moov(data: &[u8]) -> bool { has_top_level_box(data, b"moov") }
pub fn has_moof(data: &[u8]) -> bool { has_top_level_box(data, b"moof") }

fn has_top_level_box(data: &[u8], box_type: &[u8; 4]) -> bool {
    let mut offset = 0;
    while offset + 8 <= data.len() {
        let size32 = u32::from_be_bytes([
            data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
        ]) as usize;
        let actual_size = if size32 == 0 {
            data.len() - offset
        } else if size32 < 8 {
            break;
        } else {
            size32
        };
        if offset + actual_size > data.len() { break; }
        if &data[offset + 4..offset + 8] == box_type { return true; }
        offset += actual_size;
    }
    false
}

pub fn parse_track_id_from_init(data: &[u8]) -> Option<u32> {
    let moov = find_box_content(data, b"moov")?;
    let trak = find_box_content(moov, b"trak")?;
    let tkhd = find_box_content(trak, b"tkhd")?;
    if tkhd.len() < 4 { return None; }
    let offset = if tkhd[0] == 1 { 20 } else { 12 };
    if tkhd.len() < offset + 4 { return None; }
    Some(u32::from_be_bytes([
        tkhd[offset], tkhd[offset + 1], tkhd[offset + 2], tkhd[offset + 3],
    ]))
}

pub fn parse_handler_type(data: &[u8]) -> Option<String> {
    let moov = find_box_content(data, b"moov")?;
    let trak = find_box_content(moov, b"trak")?;
    let mdia = find_box_content(trak, b"mdia")?;
    let hdlr = find_box_content(mdia, b"hdlr")?;
    if hdlr.len() < 12 { return None; }
    Some(String::from_utf8_lossy(&hdlr[8..12]).to_string())
}

/// Like find_box_content but tolerates truncated boxes. Returns whatever
/// content is available even if the box header declares more data than present.
/// Used for NAL scanning where we only need the first few bytes of mdat.
fn find_mdat_content_partial(data: &[u8]) -> Option<&[u8]> {
    let mut offset = 0;
    while offset + 8 <= data.len() {
        let size = u32::from_be_bytes([
            data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
        ]) as usize;

        let actual_size = if size == 0 {
            data.len() - offset
        } else if size < 8 {
            break;
        } else {
            size
        };

        if &data[offset + 4..offset + 8] == b"mdat" {
            let content_start = offset + 8;
            let content_end = (offset + actual_size).min(data.len());
            return Some(&data[content_start..content_end]);
        }

        // For non-mdat boxes, skip even if truncated (to reach mdat after moof)
        let skip = actual_size.min(data.len() - offset);
        offset += skip;
    }
    None
}

pub fn find_box_content<'a>(data: &'a [u8], box_type: &[u8; 4]) -> Option<&'a [u8]> {
    let mut offset = 0;
    while offset + 8 <= data.len() {
        let size = u32::from_be_bytes([
            data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
        ]) as usize;

        let actual_size = if size == 0 {
            data.len() - offset
        } else if size < 8 {
            break;
        } else {
            size
        };

        if offset + actual_size > data.len() { break; }
        if &data[offset + 4..offset + 8] == box_type {
            return Some(&data[offset + 8..offset + actual_size]);
        }
        offset += actual_size;
    }
    None
}

pub fn extract_avcc_bytes(data: &[u8]) -> Option<Vec<u8>> {
    let moov = find_box_content(data, b"moov")?;
    let trak = find_box_content(moov, b"trak")?;
    let mdia = find_box_content(trak, b"mdia")?;
    let minf = find_box_content(mdia, b"minf")?;
    let stbl = find_box_content(minf, b"stbl")?;
    let stsd = find_box_content(stbl, b"stsd")?;
    if stsd.len() < 8 { return None; }
    let entry_data = &stsd[8..];
    let avc1 = find_box_content(entry_data, b"avc1")?;
    if avc1.len() <= 78 { return None; }
    let avcc = find_box_content(&avc1[78..], b"avcC")?;
    Some(avcc.to_vec())
}

pub fn extract_esds_bytes(data: &[u8]) -> Option<Vec<u8>> {
    let moov = find_box_content(data, b"moov")?;
    let trak = find_box_content(moov, b"trak")?;
    let mdia = find_box_content(trak, b"mdia")?;
    let minf = find_box_content(mdia, b"minf")?;
    let stbl = find_box_content(minf, b"stbl")?;
    let stsd = find_box_content(stbl, b"stsd")?;
    if stsd.len() < 8 { return None; }
    let entry_data = &stsd[8..];
    let mp4a = find_box_content(entry_data, b"mp4a")?;
    if mp4a.len() <= 28 { return None; }
    let esds = find_box_content(&mp4a[28..], b"esds")?;
    Some(esds.to_vec())
}

pub fn extract_video_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    let moov = find_box_content(data, b"moov")?;
    let trak = find_box_content(moov, b"trak")?;
    let tkhd = find_box_content(trak, b"tkhd")?;
    let version = tkhd[0];
    let offset = if version == 1 { 88 } else { 76 };
    if tkhd.len() < offset + 8 { return None; }
    let width = u32::from_be_bytes([tkhd[offset], tkhd[offset+1], tkhd[offset+2], tkhd[offset+3]]) >> 16;
    let height = u32::from_be_bytes([tkhd[offset+4], tkhd[offset+5], tkhd[offset+6], tkhd[offset+7]]) >> 16;
    if width > 0 && height > 0 { Some((width, height)) } else { None }
}

pub fn extract_audio_sample_rate(data: &[u8]) -> Option<u32> {
    let moov = find_box_content(data, b"moov")?;
    let trak = find_box_content(moov, b"trak")?;
    let mdia = find_box_content(trak, b"mdia")?;
    let minf = find_box_content(mdia, b"minf")?;
    let stbl = find_box_content(minf, b"stbl")?;
    let stsd = find_box_content(stbl, b"stsd")?;
    if stsd.len() < 8 { return None; }
    let entry_data = &stsd[8..];
    let mp4a = find_box_content(entry_data, b"mp4a")?;
    if mp4a.len() < 28 { return None; }
    let sr = u32::from_be_bytes([mp4a[24], mp4a[25], mp4a[26], mp4a[27]]) >> 16;
    Some(sr)
}

pub fn extract_audio_channels(data: &[u8]) -> Option<u16> {
    let moov = find_box_content(data, b"moov")?;
    let trak = find_box_content(moov, b"trak")?;
    let mdia = find_box_content(trak, b"mdia")?;
    let minf = find_box_content(mdia, b"minf")?;
    let stbl = find_box_content(minf, b"stbl")?;
    let stsd = find_box_content(stbl, b"stsd")?;
    if stsd.len() < 8 { return None; }
    let entry_data = &stsd[8..];
    let mp4a = find_box_content(entry_data, b"mp4a")?;
    if mp4a.len() < 20 { return None; }
    Some(u16::from_be_bytes([mp4a[16], mp4a[17]]))
}

/// Extract default_sample_duration from a moof fragment's tfhd box.
/// This is the real per-fragment value, unlike the trex placeholder in the init segment.
pub fn parse_tfhd_sample_duration(data: &[u8]) -> Option<u32> {
    let moof = find_box_content(data, b"moof")?;
    let traf = find_box_content(moof, b"traf")?;
    let tfhd = find_box_content(traf, b"tfhd")?;
    if tfhd.len() < 8 { return None; }

    let tf_flags = u32::from_be_bytes([0, tfhd[1], tfhd[2], tfhd[3]]);

    // default-sample-duration-present is flag 0x000008
    if tf_flags & 0x000008 == 0 {
        return None;
    }

    // Skip past: version+flags (4) + track_id (4)
    let mut off = 4 + 4;
    if tf_flags & 0x000001 != 0 { off += 8; } // base-data-offset
    if tf_flags & 0x000002 != 0 { off += 4; } // sample-description-index
    // Now at default-sample-duration
    if off + 4 <= tfhd.len() {
        let dur = u32::from_be_bytes([tfhd[off], tfhd[off+1], tfhd[off+2], tfhd[off+3]]);
        if dur > 0 { Some(dur) } else { None }
    } else {
        None
    }
}

/// Extract default_sample_duration from the trex box in the init segment (moov).
pub fn extract_default_sample_duration(data: &[u8]) -> Option<u32> {
    let moov = find_box_content(data, b"moov")?;
    let mvex = find_box_content(moov, b"mvex")?;
    let trex = find_box_content(mvex, b"trex")?;
    if trex.len() < 16 { return None; }
    let dur = u32::from_be_bytes([trex[12], trex[13], trex[14], trex[15]]);
    if dur > 0 { Some(dur) } else { None }
}

/// Rewrite the tfdt baseMediaDecodeTime to be relative (subtract `base`).
/// Returns the rewritten fragment. The caller tracks the per-track base.
pub fn rebase_decode_time(fragment: &[u8], base: u64) -> Vec<u8> {
    let mut out = fragment.to_vec();

    // Find moof box
    let mut i = 0;
    let mut moof_off = None;
    let mut moof_size = 0;
    while i + 8 <= out.len() {
        let size = u32::from_be_bytes([out[i], out[i+1], out[i+2], out[i+3]]) as usize;
        if size < 8 { break; }
        if &out[i+4..i+8] == b"moof" {
            moof_off = Some(i);
            moof_size = size;
            break;
        }
        i += size;
    }
    let moof_off = match moof_off {
        Some(o) => o,
        None => return out,
    };

    // Find traf within moof
    let moof_content_start = moof_off + 8;
    let moof_end = std::cmp::min(moof_off + moof_size, out.len());
    if moof_end <= moof_content_start {
        return out;
    }
    let mut j = moof_content_start;
    while j + 8 <= moof_end {
        let bsize = u32::from_be_bytes([out[j], out[j+1], out[j+2], out[j+3]]) as usize;
        if bsize < 8 { break; }
        if &out[j+4..j+8] == b"traf" {
            let traf_end = std::cmp::min(j + bsize, out.len());
            // Find tfdt within traf
            let mut k = j + 8;
            while k + 8 <= traf_end {
                let tsize = u32::from_be_bytes([out[k], out[k+1], out[k+2], out[k+3]]) as usize;
                if tsize < 8 { break; }
                if &out[k+4..k+8] == b"tfdt" {
                    let content_start = k + 8;
                    if content_start + 4 > out.len() { break; }
                    let version = out[content_start];
                    if version == 1 {
                        // 64-bit baseMediaDecodeTime at offset +4
                        let bdt_off = content_start + 4;
                        if bdt_off + 8 <= out.len() {
                            let old = u64::from_be_bytes([
                                out[bdt_off], out[bdt_off+1], out[bdt_off+2], out[bdt_off+3],
                                out[bdt_off+4], out[bdt_off+5], out[bdt_off+6], out[bdt_off+7],
                            ]);
                            let rebased = old.saturating_sub(base);
                            out[bdt_off..bdt_off+8].copy_from_slice(&rebased.to_be_bytes());
                        }
                    } else {
                        // 32-bit baseMediaDecodeTime at offset +4
                        let bdt_off = content_start + 4;
                        if bdt_off + 4 <= out.len() {
                            let old = u32::from_be_bytes([
                                out[bdt_off], out[bdt_off+1], out[bdt_off+2], out[bdt_off+3],
                            ]) as u64;
                            let rebased = old.saturating_sub(base) as u32;
                            out[bdt_off..bdt_off+4].copy_from_slice(&rebased.to_be_bytes());
                        }
                    }
                    return out;
                }
                k += tsize;
            }
        }
        j += bsize;
    }
    out
}

pub fn inject_trun_duration(fragment: &[u8], default_duration: u32) -> Vec<u8> {
    let mut i = 0;
    let mut moof_start = None;
    let mut moof_size = 0;
    while i + 8 <= fragment.len() {
        let size = u32::from_be_bytes([fragment[i], fragment[i+1], fragment[i+2], fragment[i+3]]) as usize;
        if size < 8 { break; }
        if &fragment[i+4..i+8] == b"moof" {
            moof_start = Some(i);
            moof_size = size;
            break;
        }
        i += size;
    }
    let moof_off = match moof_start {
        Some(o) => o,
        None => return fragment.to_vec(),
    };

    // Clamp moof_size to fragment bounds
    let moof_end = std::cmp::min(moof_off + moof_size, fragment.len());
    if moof_end <= moof_off + 8 {
        return fragment.to_vec();
    }
    let moof_content = &fragment[moof_off+8..moof_end];
    let mut j = 0;
    while j + 8 <= moof_content.len() {
        let bsize = u32::from_be_bytes([moof_content[j], moof_content[j+1], moof_content[j+2], moof_content[j+3]]) as usize;
        if bsize < 8 { break; }
        if &moof_content[j+4..j+8] == b"traf" {
            let traf_abs = moof_off + 8 + j;
            let traf_content_start = traf_abs + 8;
            let traf_end = std::cmp::min(traf_abs + bsize, fragment.len());
            if traf_end <= traf_content_start {
                break;
            }
            let traf_content = &fragment[traf_content_start..traf_end];
            let mut k = 0;
            while k + 8 <= traf_content.len() {
                let tsize = u32::from_be_bytes([traf_content[k], traf_content[k+1], traf_content[k+2], traf_content[k+3]]) as usize;
                if tsize < 8 { break; }
                if &traf_content[k+4..k+8] == b"trun" {
                    let trun_abs = traf_content_start + k;
                    return rewrite_trun_in_fragment(fragment, moof_off, moof_size, traf_abs, bsize, trun_abs, tsize, default_duration);
                }
                k += tsize;
            }
        }
        j += bsize;
    }
    fragment.to_vec()
}

fn rewrite_trun_in_fragment(
    fragment: &[u8],
    moof_off: usize, moof_size: usize,
    traf_off: usize, traf_size: usize,
    trun_off: usize, trun_size: usize,
    default_duration: u32,
) -> Vec<u8> {
    let trun_end = std::cmp::min(trun_off + trun_size, fragment.len());
    if trun_end <= trun_off + 8 { return fragment.to_vec(); }
    let trun_content = &fragment[trun_off+8..trun_end];
    if trun_content.len() < 8 { return fragment.to_vec(); }

    let flags = u32::from_be_bytes([0, trun_content[1], trun_content[2], trun_content[3]]);
    if flags & 0x100 != 0 { return fragment.to_vec(); }

    let sample_count = u32::from_be_bytes([trun_content[4], trun_content[5], trun_content[6], trun_content[7]]);
    let has_data_offset = flags & 0x001 != 0;
    let has_first_flags = flags & 0x004 != 0;
    let has_size = flags & 0x200 != 0;
    let has_flags = flags & 0x400 != 0;
    let has_cts = flags & 0x800 != 0;

    let old_per_sample = (if has_size { 4 } else { 0 })
        + (if has_flags { 4 } else { 0 })
        + (if has_cts { 4 } else { 0 });
    let header_size = 8
        + (if has_data_offset { 4 } else { 0 })
        + (if has_first_flags { 4 } else { 0 });

    let new_flags = flags | 0x100;
    let mut new_trun_content = Vec::new();
    new_trun_content.push(trun_content[0]);
    new_trun_content.push(((new_flags >> 16) & 0xff) as u8);
    new_trun_content.push(((new_flags >> 8) & 0xff) as u8);
    new_trun_content.push((new_flags & 0xff) as u8);
    let header_end = std::cmp::min(header_size, trun_content.len());
    new_trun_content.extend_from_slice(&trun_content[4..header_end]);

    let samples_start = header_size;
    for s in 0..sample_count as usize {
        let off = samples_start + s * old_per_sample;
        new_trun_content.extend_from_slice(&default_duration.to_be_bytes());
        if off + old_per_sample <= trun_content.len() {
            new_trun_content.extend_from_slice(&trun_content[off..off+old_per_sample]);
        }
    }

    let new_trun_box_size = (8 + new_trun_content.len()) as u32;
    let mut new_trun = Vec::with_capacity(new_trun_box_size as usize);
    new_trun.extend_from_slice(&new_trun_box_size.to_be_bytes());
    new_trun.extend_from_slice(b"trun");
    new_trun.extend_from_slice(&new_trun_content);

    let size_delta = new_trun.len() as i64 - trun_size as i64;

    let trun_end_off = std::cmp::min(trun_off + trun_size, fragment.len());
    let mut result = Vec::with_capacity((fragment.len() as i64 + size_delta) as usize);
    result.extend_from_slice(&fragment[..trun_off]);
    result.extend_from_slice(&new_trun);
    if trun_end_off < fragment.len() {
        result.extend_from_slice(&fragment[trun_end_off..]);
    }

    let new_moof_size = (moof_size as i64 + size_delta) as u32;
    result[moof_off..moof_off+4].copy_from_slice(&new_moof_size.to_be_bytes());

    let new_traf_size = (traf_size as i64 + size_delta) as u32;
    result[traf_off..traf_off+4].copy_from_slice(&new_traf_size.to_be_bytes());

    if has_data_offset {
        let do_off = trun_off + 8 + 8;
        let old_data_offset = i32::from_be_bytes([result[do_off], result[do_off+1], result[do_off+2], result[do_off+3]]);
        let new_data_offset = (old_data_offset as i64 + size_delta) as i32;
        result[do_off..do_off+4].copy_from_slice(&new_data_offset.to_be_bytes());
    }

    result
}