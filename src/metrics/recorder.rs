use prometheus::{Counter, Gauge, Opts};
use serde::Serialize;
use std::sync::OnceLock;
use std::time::Duration;

#[derive(Default, Serialize)]
pub(crate) struct RecorderSnapshot {
    active_recordings: u64,
    segments_written_total: u64,
    bytes_written_total: u64,
    segment_write_seconds_total: f64,
    segment_write_operations_total: u64,
    segment_processing_seconds_total: f64,
    segment_processing_operations_total: u64,
    segment_write_failures_total: u64,
    recording_failures_total: u64,
    prepare_seconds_total: f64,
    prepare_operations_total: u64,
    prepare_failures_total: u64,
}

struct RecorderMetrics {
    active_recordings: Gauge,
    segments_written: Counter,
    bytes_written: Counter,
    segment_write_seconds: Counter,
    segment_write_operations: Counter,
    segment_processing_seconds: Counter,
    segment_processing_operations: Counter,
    segment_write_failures: Counter,
    recording_failures: Counter,
    prepare_seconds: Counter,
    prepare_operations: Counter,
    prepare_failures: Counter,
}

static METRICS: OnceLock<RecorderMetrics> = OnceLock::new();

pub(crate) fn init() {
    if METRICS.get().is_some() {
        return;
    }
    let metrics = match build_metrics() {
        Ok(metrics) => metrics,
        Err(error) => {
            log::error!("failed to initialize recorder metrics: {error}");
            return;
        }
    };
    let _ = METRICS.set(metrics);
}

pub(crate) fn snapshot() -> RecorderSnapshot {
    let Some(metrics) = METRICS.get() else {
        return RecorderSnapshot::default();
    };
    RecorderSnapshot {
        active_recordings: metrics.active_recordings.get().max(0.0) as u64,
        segments_written_total: metrics.segments_written.get().max(0.0) as u64,
        bytes_written_total: metrics.bytes_written.get().max(0.0) as u64,
        segment_write_seconds_total: metrics.segment_write_seconds.get(),
        segment_write_operations_total: metrics.segment_write_operations.get().max(0.0) as u64,
        segment_processing_seconds_total: metrics.segment_processing_seconds.get(),
        segment_processing_operations_total: metrics.segment_processing_operations.get().max(0.0) as u64,
        segment_write_failures_total: metrics.segment_write_failures.get().max(0.0) as u64,
        recording_failures_total: metrics.recording_failures.get().max(0.0) as u64,
        prepare_seconds_total: metrics.prepare_seconds.get(),
        prepare_operations_total: metrics.prepare_operations.get().max(0.0) as u64,
        prepare_failures_total: metrics.prepare_failures.get().max(0.0) as u64,
    }
}

pub(crate) struct ActiveRecording;

impl ActiveRecording {
    pub(crate) fn start() -> Self {
        if let Some(metrics) = METRICS.get() {
            metrics.active_recordings.inc();
        }
        Self
    }
}

impl Drop for ActiveRecording {
    fn drop(&mut self) {
        if let Some(metrics) = METRICS.get() {
            metrics.active_recordings.dec();
        }
    }
}

pub(crate) fn preparation_finished(duration: Duration, failed: bool) {
    if let Some(metrics) = METRICS.get() {
        metrics.prepare_seconds.inc_by(duration.as_secs_f64());
        metrics.prepare_operations.inc();
        if failed {
            metrics.prepare_failures.inc();
        }
    }
}

pub(crate) fn segment_written(bytes: usize, duration: Duration) {
    if let Some(metrics) = METRICS.get() {
        metrics.segments_written.inc();
        metrics.bytes_written.inc_by(bytes as f64);
        metrics.segment_write_seconds.inc_by(duration.as_secs_f64());
        metrics.segment_write_operations.inc();
    }
}

pub(crate) fn segment_write_failed() {
    if let Some(metrics) = METRICS.get() {
        metrics.segment_write_failures.inc();
    }
}

pub(crate) fn segment_processed(duration: Duration) {
    if let Some(metrics) = METRICS.get() {
        metrics.segment_processing_seconds.inc_by(duration.as_secs_f64());
        metrics.segment_processing_operations.inc();
    }
}

pub(crate) fn recording_failed() {
    if let Some(metrics) = METRICS.get() {
        metrics.recording_failures.inc();
    }
}

fn build_metrics() -> Result<RecorderMetrics, prometheus::Error> {
    let metrics = RecorderMetrics {
        active_recordings: register_gauge(
            "shig_recorder_active_recordings",
            "Number of active HLS recordings.",
        )?,
        segments_written: register_counter(
            "shig_recorder_segments_written_total",
            "Number of HLS media segments written by the recorder.",
        )?,
        bytes_written: register_counter(
            "shig_recorder_bytes_written_total",
            "Media segment bytes written by the recorder.",
        )?,
        segment_write_seconds: register_counter(
            "shig_recorder_segment_write_seconds_total",
            "Cumulative time spent writing HLS media segments.",
        )?,
        segment_write_operations: register_counter(
            "shig_recorder_segment_write_operations_total",
            "Number of successful HLS media segment writes.",
        )?,
        segment_processing_seconds: register_counter(
            "shig_recorder_segment_processing_seconds_total",
            "Cumulative time processing an HLS segment before and during persistence.",
        )?,
        segment_processing_operations: register_counter(
            "shig_recorder_segment_processing_operations_total",
            "Number of successfully processed HLS media segments.",
        )?,
        segment_write_failures: register_counter(
            "shig_recorder_segment_write_failures_total",
            "Number of failed HLS media segment writes.",
        )?,
        recording_failures: register_counter(
            "shig_recorder_recording_failures_total",
            "Number of failed HLS recording jobs.",
        )?,
        prepare_seconds: register_counter(
            "shig_recorder_prepare_seconds_total",
            "Cumulative time spent preparing HLS recordings.",
        )?,
        prepare_operations: register_counter(
            "shig_recorder_prepare_operations_total",
            "Number of HLS recording preparation attempts.",
        )?,
        prepare_failures: register_counter(
            "shig_recorder_prepare_failures_total",
            "Number of failed HLS recording preparation attempts.",
        )?,
    };
    Ok(metrics)
}

fn register_counter(name: &str, help: &str) -> Result<Counter, prometheus::Error> {
    let counter = Counter::with_opts(Opts::new(name, help))?;
    prometheus::default_registry().register(Box::new(counter.clone()))?;
    Ok(counter)
}

fn register_gauge(name: &str, help: &str) -> Result<Gauge, prometheus::Error> {
    let gauge = Gauge::with_opts(Opts::new(name, help))?;
    prometheus::default_registry().register(Box::new(gauge.clone()))?;
    Ok(gauge)
}
