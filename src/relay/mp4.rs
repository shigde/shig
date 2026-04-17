mod parser;

pub use parser::{
    has_moof, has_moov,
    parse_base_decode_time, parse_codec_from_init, parse_handler_type,
    parse_moof_mdat_ranges, parse_timescale,
    parse_track_id_from_init,
    extract_avcc_bytes, extract_esds_bytes,
    extract_video_dimensions, extract_audio_sample_rate, extract_audio_channels,
    extract_default_sample_duration, parse_tfhd_sample_duration,
    rebase_decode_time, inject_trun_duration,
    fragment_starts_with_idr,
};