use moq_lite::{GroupProducer, TrackProducer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackType {
    Video,
    Audio,
}

pub struct TrackState {
    pub track: TrackProducer,
    pub timescale: u32,
    pub default_sample_duration: Option<u32>,
    pub group: Option<GroupProducer>,
    /// First baseMediaDecodeTime seen for this track (used to rebase to 0).
    pub time_base: Option<u64>,
    pub track_type: TrackType,
    /// Signal from HTTP ingest: new segment (HTTP PUT) started, force new group.
    pub new_segment: bool,
    /// Last baseMediaDecodeTime seen (pre-rebase).
    pub last_bdt: Option<u64>,
    /// Last fragment's duration in timescale ticks (estimated from BDT deltas).
    pub last_frag_duration: Option<u64>,
}