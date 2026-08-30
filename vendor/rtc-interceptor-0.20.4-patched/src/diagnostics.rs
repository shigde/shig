use prometheus::{default_registry, IntCounterVec, Opts};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

static NACK_RESPONDER_DIAGNOSTICS_ENABLED: AtomicBool = AtomicBool::new(false);
static NACK_RESPONDER_EVENTS: OnceLock<IntCounterVec> = OnceLock::new();
static NACK_RESPONDER_RTX: OnceLock<IntCounterVec> = OnceLock::new();

/// Enables or disables NACK responder diagnostics for all interceptor chains in this process.
pub fn set_nack_responder_diagnostics_enabled(enabled: bool) {
    NACK_RESPONDER_DIAGNOSTICS_ENABLED.store(enabled, Ordering::Relaxed);
    if enabled {
        nack_responder_events();
        nack_responder_rtx();
    }
}

pub(crate) fn inc_nack_responder_event(
    event: &'static str,
    media: &'static str,
    detail: &'static str,
) {
    if !NACK_RESPONDER_DIAGNOSTICS_ENABLED.load(Ordering::Relaxed) {
        return;
    }
    nack_responder_events()
        .with_label_values(&[event, media, detail])
        .inc();
}

pub(crate) fn inc_nack_responder_rtx(
    media_ssrc: u32,
    rtx_ssrc: u32,
    original_payload_type: u8,
    rtx_payload_type: u8,
) {
    if !NACK_RESPONDER_DIAGNOSTICS_ENABLED.load(Ordering::Relaxed) {
        return;
    }

    let media_ssrc = media_ssrc.to_string();
    let rtx_ssrc = rtx_ssrc.to_string();
    let original_payload_type = original_payload_type.to_string();
    let rtx_payload_type = rtx_payload_type.to_string();
    nack_responder_rtx()
        .with_label_values(&[
            &media_ssrc,
            &rtx_ssrc,
            &original_payload_type,
            &rtx_payload_type,
        ])
        .inc();
}

fn nack_responder_events() -> &'static IntCounterVec {
    NACK_RESPONDER_EVENTS.get_or_init(|| {
        let counter = IntCounterVec::new(
            Opts::new(
                "shig_rtc_nack_responder_events_total",
                "NACK responder diagnostics observed inside rtc-interceptor.",
            ),
            &["event", "media", "detail"],
        )
        .expect("valid nack responder diagnostics counter");
        let _ = default_registry().register(Box::new(counter.clone()));
        counter
    })
}

fn nack_responder_rtx() -> &'static IntCounterVec {
    NACK_RESPONDER_RTX.get_or_init(|| {
        let counter = IntCounterVec::new(
            Opts::new(
                "shig_rtc_nack_responder_rtx_total",
                "RTX packets queued by the NACK responder with SSRC and payload type mapping.",
            ),
            &[
                "media_ssrc",
                "rtx_ssrc",
                "original_payload_type",
                "rtx_payload_type",
            ],
        )
        .expect("valid nack responder rtx diagnostics counter");
        let _ = default_registry().register(Box::new(counter.clone()));
        counter
    })
}
