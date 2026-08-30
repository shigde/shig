use prometheus::{
    default_registry, Encoder, GaugeVec, HistogramOpts, HistogramVec, IntCounterVec, IntGaugeVec,
    Opts,
    TextEncoder,
};
use std::collections::HashMap;
use std::sync::OnceLock;

static RTP_PACKETS: OnceLock<IntCounterVec> = OnceLock::new();
static RTP_BYTES: OnceLock<IntCounterVec> = OnceLock::new();
static RTP_DROPPED: OnceLock<IntCounterVec> = OnceLock::new();
static RTCP_PACKETS: OnceLock<IntCounterVec> = OnceLock::new();
static RTCP_DROPPED: OnceLock<IntCounterVec> = OnceLock::new();
static RTCP_KEYFRAME_REQUESTS: OnceLock<IntCounterVec> = OnceLock::new();
static RTC_PACKET_IO: OnceLock<IntCounterVec> = OnceLock::new();
static RTC_PACKET_IO_BYTES: OnceLock<IntCounterVec> = OnceLock::new();
static RTC_RTP_PACKET_IO: OnceLock<IntCounterVec> = OnceLock::new();
static RTC_RTP_PACKET_IO_BYTES: OnceLock<IntCounterVec> = OnceLock::new();
static RTC_UDP_SEND_ERRORS: OnceLock<IntCounterVec> = OnceLock::new();
static RTC_NACK_CACHE: OnceLock<IntCounterVec> = OnceLock::new();
static RECONCILE_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();
static RECONCILE_CHANGES: OnceLock<IntCounterVec> = OnceLock::new();
static RECONCILE_DURATION: OnceLock<HistogramVec> = OnceLock::new();
static RTP_FORWARD_DELAY: OnceLock<HistogramVec> = OnceLock::new();
static ENGINE_STATE: OnceLock<IntGaugeVec> = OnceLock::new();
static PEER_CONNECTION_STATS: OnceLock<GaugeVec> = OnceLock::new();

const PEER_CONNECTION_ROLES: &[&str] = &["publish", "subscribe"];
const PEER_CONNECTION_PURPOSES: &[&str] = &["participant", "stream", "connection", "unknown"];
const PEER_CONNECTION_MEDIA: &[&str] = &["audio", "video", "transport"];
const PEER_CONNECTION_METRICS: &[&str] = &[
    "outbound_bytes_sent",
    "outbound_packets_sent",
    "outbound_nack_count",
    "outbound_pli_count",
    "outbound_fir_count",
    "remote_inbound_packets_lost",
    "remote_inbound_fraction_lost_max",
    "remote_inbound_rtt_seconds_max",
    "inbound_bytes_received",
    "inbound_packets_received",
    "inbound_packets_lost",
    "inbound_jitter_seconds_max",
    "inbound_nack_count",
    "inbound_pli_count",
    "inbound_fir_count",
    "candidate_pair_current_rtt_seconds_max",
    "candidate_pair_available_outgoing_bitrate",
];

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub(crate) struct PeerConnectionMetricKey {
    role: &'static str,
    purpose: &'static str,
    media: &'static str,
    metric: &'static str,
}

#[derive(Default)]
pub(crate) struct PeerConnectionStats {
    values: HashMap<PeerConnectionMetricKey, f64>,
}

impl PeerConnectionStats {
    pub(crate) fn add(
        &mut self,
        role: &'static str,
        purpose: &'static str,
        media: &'static str,
        metric: &'static str,
        value: f64,
    ) {
        *self
            .values
            .entry(PeerConnectionMetricKey {
                role,
                purpose,
                media,
                metric,
            })
            .or_default() += value;
    }

    pub(crate) fn max(
        &mut self,
        role: &'static str,
        purpose: &'static str,
        media: &'static str,
        metric: &'static str,
        value: f64,
    ) {
        self.values
            .entry(PeerConnectionMetricKey {
                role,
                purpose,
                media,
                metric,
            })
            .and_modify(|current| *current = current.max(value))
            .or_insert(value);
    }
}

pub(crate) fn init() {
    rtp_packets();
    rtp_bytes();
    rtp_dropped();
    rtcp_packets();
    rtcp_dropped();
    rtcp_keyframe_requests();
    rtc_packet_io();
    rtc_packet_io_bytes();
    rtc_rtp_packet_io();
    rtc_rtp_packet_io_bytes();
    rtc_udp_send_errors();
    rtc_nack_cache();
    reconcile_total();
    reconcile_changes();
    reconcile_duration();
    rtp_forward_delay();
    engine_state();
    peer_connection_stats();
}

pub(crate) fn render() -> Result<String, prometheus::Error> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer)?;
    Ok(String::from_utf8_lossy(&buffer).into_owned())
}

pub(crate) fn inc_rtp_in(bytes: usize) {
    rtp_packets().with_label_values(&["in"]).inc();
    rtp_bytes()
        .with_label_values(&["in"])
        .inc_by(bytes as u64);
}

pub(crate) fn inc_rtp_forwarded(bytes: usize) {
    rtp_packets().with_label_values(&["forwarded"]).inc();
    rtp_bytes()
        .with_label_values(&["forwarded"])
        .inc_by(bytes as u64);
}

pub(crate) fn inc_rtp_dropped(reason: &'static str) {
    rtp_dropped().with_label_values(&[reason]).inc();
}

pub(crate) fn inc_rtcp_in(kind: &'static str) {
    rtcp_packets().with_label_values(&["in", kind]).inc();
}

pub(crate) fn inc_rtcp_keyframe_request(outcome: &'static str) {
    rtcp_keyframe_requests().with_label_values(&[outcome]).inc();
}

pub(crate) fn inc_rtcp_dropped(reason: &'static str) {
    rtcp_dropped().with_label_values(&[reason]).inc();
}

pub(crate) fn inc_rtc_peer_connection_out(
    role: &'static str,
    packet_kind: &'static str,
    bytes: usize,
) {
    rtc_packet_io()
        .with_label_values(&["peer_connection_out", role, packet_kind])
        .inc();
    rtc_packet_io_bytes()
        .with_label_values(&["peer_connection_out", role, packet_kind])
        .inc_by(bytes as u64);
}

pub(crate) fn inc_rtc_peer_connection_rtp_out(
    role: &'static str,
    ssrc: u32,
    payload_type: u8,
    bytes: usize,
) {
    let ssrc = ssrc.to_string();
    let payload_type = payload_type.to_string();
    rtc_rtp_packet_io()
        .with_label_values(&["peer_connection_out", role, &ssrc, &payload_type])
        .inc();
    rtc_rtp_packet_io_bytes()
        .with_label_values(&["peer_connection_out", role, &ssrc, &payload_type])
        .inc_by(bytes as u64);
}

pub(crate) fn inc_rtc_udp_out(packet_kind: &'static str, bytes: usize) {
    rtc_packet_io()
        .with_label_values(&["udp_out", "all", packet_kind])
        .inc();
    rtc_packet_io_bytes()
        .with_label_values(&["udp_out", "all", packet_kind])
        .inc_by(bytes as u64);
}

pub(crate) fn inc_rtc_udp_send_error(packet_kind: &'static str) {
    rtc_udp_send_errors()
        .with_label_values(&[packet_kind])
        .inc();
}

pub(crate) fn inc_rtc_nack_cache(
    role: &'static str,
    event: &'static str,
    media: &'static str,
    detail: &'static str,
) {
    rtc_nack_cache()
        .with_label_values(&[role, event, media, detail])
        .inc();
}

pub(crate) fn observe_reconcile(duration_seconds: f64, added_routes: u64, removed_routes: u64) {
    reconcile_total().with_label_values(&["run"]).inc();
    reconcile_changes()
        .with_label_values(&["added"])
        .inc_by(added_routes);
    reconcile_changes()
        .with_label_values(&["removed"])
        .inc_by(removed_routes);
    reconcile_duration()
        .with_label_values(&["seconds"])
        .observe(duration_seconds);
}

pub(crate) fn observe_rtp_forward_delay(
    media: &'static str,
    subscriber_count: usize,
    duration_seconds: f64,
) {
    let subscriber_count = subscriber_count.to_string();
    rtp_forward_delay()
        .with_label_values(&[media, &subscriber_count])
        .observe(duration_seconds);
}

pub(crate) fn set_engine_state(
    engine: u64,
    lobbies: usize,
    endpoints: usize,
    publishers: usize,
    subscribers: usize,
    routes: usize,
) {
    let engine = engine.to_string();
    let gauges = engine_state();
    gauges
        .with_label_values(&[&engine, "lobbies"])
        .set(lobbies as i64);
    gauges
        .with_label_values(&[&engine, "endpoints"])
        .set(endpoints as i64);
    gauges
        .with_label_values(&[&engine, "publishers"])
        .set(publishers as i64);
    gauges
        .with_label_values(&[&engine, "subscribers"])
        .set(subscribers as i64);
    gauges
        .with_label_values(&[&engine, "routes"])
        .set(routes as i64);
}

pub(crate) fn set_peer_connection_stats(engine: u64, stats: &PeerConnectionStats) {
    let engine = engine.to_string();
    let gauges = peer_connection_stats();

    for role in PEER_CONNECTION_ROLES {
        for purpose in PEER_CONNECTION_PURPOSES {
            for media in PEER_CONNECTION_MEDIA {
                for metric in PEER_CONNECTION_METRICS {
                    let _ = gauges.remove_label_values(&[&engine, role, purpose, media, metric]);
                }
            }
        }
    }

    for (key, value) in &stats.values {
        gauges
            .with_label_values(&[&engine, key.role, key.purpose, key.media, key.metric])
            .set(*value);
    }
}

fn rtp_packets() -> &'static IntCounterVec {
    RTP_PACKETS.get_or_init(|| {
        register_int_counter_vec(
            "shig_rtc_rtp_packets_total",
            "Total RTP packets observed by the SFU media router.",
            &["direction"],
        )
    })
}

fn rtp_bytes() -> &'static IntCounterVec {
    RTP_BYTES.get_or_init(|| {
        register_int_counter_vec(
            "shig_rtc_rtp_bytes_total",
            "Total RTP bytes observed by the SFU media router.",
            &["direction"],
        )
    })
}

fn rtp_dropped() -> &'static IntCounterVec {
    RTP_DROPPED.get_or_init(|| {
        register_int_counter_vec(
            "shig_rtc_rtp_dropped_total",
            "Total RTP packets dropped by the SFU media router.",
            &["reason"],
        )
    })
}

fn rtcp_packets() -> &'static IntCounterVec {
    RTCP_PACKETS.get_or_init(|| {
        register_int_counter_vec(
            "shig_rtc_rtcp_packets_total",
            "Total RTCP packets observed by the SFU media router.",
            &["direction", "kind"],
        )
    })
}

fn rtcp_dropped() -> &'static IntCounterVec {
    RTCP_DROPPED.get_or_init(|| {
        register_int_counter_vec(
            "shig_rtc_rtcp_dropped_total",
            "Total RTCP packets dropped by the SFU media router.",
            &["reason"],
        )
    })
}

fn rtcp_keyframe_requests() -> &'static IntCounterVec {
    RTCP_KEYFRAME_REQUESTS.get_or_init(|| {
        register_int_counter_vec(
            "shig_rtc_rtcp_keyframe_requests_total",
            "Total RTCP PLI/FIR keyframe requests handled by the SFU media router.",
            &["outcome"],
        )
    })
}

fn rtc_packet_io() -> &'static IntCounterVec {
    RTC_PACKET_IO.get_or_init(|| {
        register_int_counter_vec(
            "shig_rtc_packet_io_total",
            "Total RTC transport packets emitted by peer connections and queued for UDP send.",
            &["stage", "role", "packet_kind"],
        )
    })
}

fn rtc_packet_io_bytes() -> &'static IntCounterVec {
    RTC_PACKET_IO_BYTES.get_or_init(|| {
        register_int_counter_vec(
            "shig_rtc_packet_io_bytes_total",
            "Total RTC transport bytes emitted by peer connections and queued for UDP send.",
            &["stage", "role", "packet_kind"],
        )
    })
}

fn rtc_rtp_packet_io() -> &'static IntCounterVec {
    RTC_RTP_PACKET_IO.get_or_init(|| {
        register_int_counter_vec(
            "shig_rtc_rtp_packet_io_total",
            "Total RTC RTP packets emitted by peer connections with SSRC and payload type.",
            &["stage", "role", "ssrc", "payload_type"],
        )
    })
}

fn rtc_rtp_packet_io_bytes() -> &'static IntCounterVec {
    RTC_RTP_PACKET_IO_BYTES.get_or_init(|| {
        register_int_counter_vec(
            "shig_rtc_rtp_packet_io_bytes_total",
            "Total RTC RTP bytes emitted by peer connections with SSRC and payload type.",
            &["stage", "role", "ssrc", "payload_type"],
        )
    })
}

fn rtc_udp_send_errors() -> &'static IntCounterVec {
    RTC_UDP_SEND_ERRORS.get_or_init(|| {
        register_int_counter_vec(
            "shig_rtc_udp_send_errors_total",
            "Total RTC UDP send errors by packet kind.",
            &["packet_kind"],
        )
    })
}

fn rtc_nack_cache() -> &'static IntCounterVec {
    RTC_NACK_CACHE.get_or_init(|| {
        register_int_counter_vec(
            "shig_rtc_nack_cache_events_total",
            "NACK cache diagnostics observed at the RTC interceptor boundary.",
            &["role", "event", "media", "detail"],
        )
    })
}

fn reconcile_total() -> &'static IntCounterVec {
    RECONCILE_TOTAL.get_or_init(|| {
        register_int_counter_vec(
            "shig_rtc_reconcile_total",
            "Total SFU media route reconcile runs.",
            &["status"],
        )
    })
}

fn reconcile_changes() -> &'static IntCounterVec {
    RECONCILE_CHANGES.get_or_init(|| {
        register_int_counter_vec(
            "shig_rtc_reconcile_route_changes_total",
            "Total forwarding route changes made during SFU media route reconcile.",
            &["change"],
        )
    })
}

fn reconcile_duration() -> &'static HistogramVec {
    RECONCILE_DURATION.get_or_init(|| {
        let histogram = HistogramVec::new(
            HistogramOpts::new(
                "shig_rtc_reconcile_duration_seconds",
                "Duration of SFU media route reconcile runs.",
            )
            .buckets(vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5]),
            &["unit"],
        )
        .expect("valid reconcile duration histogram");
        default_registry()
            .register(Box::new(histogram.clone()))
            .expect("register reconcile duration histogram");
        histogram
    })
}

fn rtp_forward_delay() -> &'static HistogramVec {
    RTP_FORWARD_DELAY.get_or_init(|| {
        let histogram = HistogramVec::new(
            HistogramOpts::new(
                "shig_rtc_rtp_forward_delay_seconds",
                "Time spent forwarding one inbound RTP packet through the SFU media router.",
            )
            .buckets(vec![
                0.0001, 0.00025, 0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1,
                0.25, 0.5, 1.0,
            ]),
            &["media", "subscriber_count"],
        )
        .expect("valid RTP forward delay histogram");
        default_registry()
            .register(Box::new(histogram.clone()))
            .expect("register RTP forward delay histogram");
        histogram
    })
}

fn engine_state() -> &'static IntGaugeVec {
    ENGINE_STATE.get_or_init(|| {
        let gauge = IntGaugeVec::new(
            Opts::new(
                "shig_rtc_engine_state",
                "Current SFU media engine state.",
            ),
            &["engine", "kind"],
        )
        .expect("valid engine state gauge");
        default_registry()
            .register(Box::new(gauge.clone()))
            .expect("register engine state gauge");
        gauge
    })
}

fn peer_connection_stats() -> &'static GaugeVec {
    PEER_CONNECTION_STATS.get_or_init(|| {
        let gauge = GaugeVec::new(
            Opts::new(
                "shig_rtc_peer_connection_stats",
                "Aggregated WebRTC PeerConnection stats exported by rtc.",
            ),
            &["engine", "role", "purpose", "media", "metric"],
        )
        .expect("valid peer connection stats gauge");
        default_registry()
            .register(Box::new(gauge.clone()))
            .expect("register peer connection stats gauge");
        gauge
    })
}

fn register_int_counter_vec(name: &str, help: &str, labels: &[&str]) -> IntCounterVec {
    let counter = IntCounterVec::new(Opts::new(name, help), labels)
        .expect("valid prometheus counter");
    default_registry()
        .register(Box::new(counter.clone()))
        .expect("register prometheus counter");
    counter
}
