use prometheus::Gauge;
use serde::Serialize;
use std::{
    collections::VecDeque,
    sync::{Arc, OnceLock, RwLock},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use sysinfo::{DiskRefreshKind, Disks, Networks, System};

const SAMPLE_INTERVAL: Duration = Duration::from_secs(5);
const HISTORY_SAMPLES: usize = 120;

#[derive(Clone, Default, Serialize)]
pub(crate) struct SystemSnapshot {
    sampled_at_unix_ms: u64,
    cpu_usage_percent: f32,
    memory_used_bytes: u64,
    memory_total_bytes: u64,
    network_received_bytes_per_second: f64,
    network_transmitted_bytes_per_second: f64,
    disk_read_bytes_per_second: f64,
    disk_written_bytes_per_second: f64,
    root_disk_used_bytes: Option<u64>,
    root_disk_total_bytes: Option<u64>,
    history: Vec<SystemHistoryPoint>,
}

#[derive(Clone, Serialize)]
struct SystemHistoryPoint {
    sampled_at_unix_ms: u64,
    cpu_usage_percent: f32,
    memory_used_bytes: u64,
    memory_total_bytes: u64,
    network_received_bytes_per_second: f64,
    network_transmitted_bytes_per_second: f64,
    disk_read_bytes_per_second: f64,
    disk_written_bytes_per_second: f64,
}

struct SystemGauges {
    cpu_usage_percent: Gauge,
    memory_used_bytes: Gauge,
    memory_total_bytes: Gauge,
    network_received_bytes_per_second: Gauge,
    network_transmitted_bytes_per_second: Gauge,
    disk_read_bytes_per_second: Gauge,
    disk_written_bytes_per_second: Gauge,
    root_disk_used_bytes: Gauge,
    root_disk_total_bytes: Gauge,
}

struct SharedMetrics {
    latest: RwLock<LatestMetrics>,
    gauges: SystemGauges,
}

#[derive(Default)]
struct LatestMetrics {
    current: SystemSnapshot,
    history: VecDeque<SystemHistoryPoint>,
}

static METRICS: OnceLock<Arc<SharedMetrics>> = OnceLock::new();

pub(crate) fn init() {
    let shared = match build_shared_metrics() {
        Ok(shared) => Arc::new(shared),
        Err(error) => {
            log::error!("failed to initialize system metrics: {error}");
            return;
        }
    };

    if METRICS.set(shared.clone()).is_err() {
        return;
    }

    let spawn_result = thread::Builder::new()
        .name("system-metrics".into())
        .spawn(move || sample_loop(shared));
    if let Err(error) = spawn_result {
        log::error!("failed to start system metrics sampler: {error}");
    }
}

pub(crate) fn snapshot() -> SystemSnapshot {
    let Some(metrics) = METRICS.get() else {
        return SystemSnapshot::default();
    };
    let Ok(latest) = metrics.latest.read() else {
        return SystemSnapshot::default();
    };
    let mut snapshot = latest.current.clone();
    snapshot.history = latest.history.iter().cloned().collect();
    snapshot
}

fn build_shared_metrics() -> Result<SharedMetrics, prometheus::Error> {
    let gauges = SystemGauges {
        cpu_usage_percent: register_gauge(
            "shig_system_cpu_usage_percent",
            "System-wide CPU usage percentage.",
        )?,
        memory_used_bytes: register_gauge(
            "shig_system_memory_used_bytes",
            "System-wide used memory in bytes.",
        )?,
        memory_total_bytes: register_gauge(
            "shig_system_memory_total_bytes",
            "System-wide total memory in bytes.",
        )?,
        network_received_bytes_per_second: register_gauge(
            "shig_system_network_received_bytes_per_second",
            "System-wide received network traffic in bytes per second.",
        )?,
        network_transmitted_bytes_per_second: register_gauge(
            "shig_system_network_transmitted_bytes_per_second",
            "System-wide transmitted network traffic in bytes per second.",
        )?,
        disk_read_bytes_per_second: register_gauge(
            "shig_system_disk_read_bytes_per_second",
            "System-wide disk reads in bytes per second.",
        )?,
        disk_written_bytes_per_second: register_gauge(
            "shig_system_disk_written_bytes_per_second",
            "System-wide disk writes in bytes per second.",
        )?,
        root_disk_used_bytes: register_gauge(
            "shig_system_root_disk_used_bytes",
            "Used space on the root filesystem in bytes.",
        )?,
        root_disk_total_bytes: register_gauge(
            "shig_system_root_disk_total_bytes",
            "Total size of the root filesystem in bytes.",
        )?,
    };

    Ok(SharedMetrics {
        latest: RwLock::new(LatestMetrics::default()),
        gauges,
    })
}

fn register_gauge(name: &str, help: &str) -> Result<Gauge, prometheus::Error> {
    let gauge = Gauge::new(name, help)?;
    prometheus::default_registry().register(Box::new(gauge.clone()))?;
    Ok(gauge)
}

fn sample_loop(shared: Arc<SharedMetrics>) {
    // Keep the same sysinfo objects between samples. CPU usage and I/O rates are
    // calculated from deltas, and refreshing only the needed categories avoids
    // loading process lists or other unrelated system data.
    let mut system = System::new();
    system.refresh_cpu_usage();
    system.refresh_memory();

    let mut networks = Networks::new_with_refreshed_list();
    let mut disks = Disks::new_with_refreshed_list_specifics(
        DiskRefreshKind::nothing().with_storage().with_io_usage(),
    );
    let mut previous_sample = Instant::now();

    loop {
        thread::sleep(SAMPLE_INTERVAL);
        system.refresh_cpu_usage();
        system.refresh_memory();
        networks.refresh(true);
        disks.refresh(false);

        let elapsed = previous_sample.elapsed().as_secs_f64().max(0.001);
        previous_sample = Instant::now();
        let (network_received, network_transmitted) = networks.iter().fold(
            (0_u64, 0_u64),
            |(received, transmitted), (_, data)| {
                (
                    received.saturating_add(data.received()),
                    transmitted.saturating_add(data.transmitted()),
                )
            },
        );
        let (disk_read, disk_written) = disks.iter().fold(
            (0_u64, 0_u64),
            |(read, written), disk| {
                let usage = disk.usage();
                (
                    read.saturating_add(usage.read_bytes),
                    written.saturating_add(usage.written_bytes),
                )
            },
        );
        let root_disk = disks
            .iter()
            .find(|disk| disk.mount_point().to_string_lossy() == "/");
        let (root_disk_used_bytes, root_disk_total_bytes) = root_disk
            .map(|disk| {
                (
                    Some(disk.total_space().saturating_sub(disk.available_space())),
                    Some(disk.total_space()),
                )
            })
            .unwrap_or((None, None));

        let snapshot = SystemSnapshot {
            sampled_at_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .min(u64::MAX as u128) as u64,
            cpu_usage_percent: system.global_cpu_usage(),
            memory_used_bytes: system.used_memory(),
            memory_total_bytes: system.total_memory(),
            network_received_bytes_per_second: network_received as f64 / elapsed,
            network_transmitted_bytes_per_second: network_transmitted as f64 / elapsed,
            disk_read_bytes_per_second: disk_read as f64 / elapsed,
            disk_written_bytes_per_second: disk_written as f64 / elapsed,
            root_disk_used_bytes,
            root_disk_total_bytes,
            history: Vec::new(),
        };

        update_gauges(&shared.gauges, &snapshot);
        match shared.latest.write() {
            Ok(mut latest) => {
                latest.history.push_back(SystemHistoryPoint::from(&snapshot));
                if latest.history.len() > HISTORY_SAMPLES {
                    latest.history.pop_front();
                }
                latest.current = snapshot;
            }
            Err(error) => {
                log::error!("system metrics snapshot lock is poisoned: {error}");
                return;
            }
        }
    }
}

impl From<&SystemSnapshot> for SystemHistoryPoint {
    fn from(snapshot: &SystemSnapshot) -> Self {
        Self {
            sampled_at_unix_ms: snapshot.sampled_at_unix_ms,
            cpu_usage_percent: snapshot.cpu_usage_percent,
            memory_used_bytes: snapshot.memory_used_bytes,
            memory_total_bytes: snapshot.memory_total_bytes,
            network_received_bytes_per_second: snapshot.network_received_bytes_per_second,
            network_transmitted_bytes_per_second: snapshot.network_transmitted_bytes_per_second,
            disk_read_bytes_per_second: snapshot.disk_read_bytes_per_second,
            disk_written_bytes_per_second: snapshot.disk_written_bytes_per_second,
        }
    }
}

fn update_gauges(gauges: &SystemGauges, snapshot: &SystemSnapshot) {
    gauges
        .cpu_usage_percent
        .set(snapshot.cpu_usage_percent as f64);
    gauges
        .memory_used_bytes
        .set(snapshot.memory_used_bytes as f64);
    gauges
        .memory_total_bytes
        .set(snapshot.memory_total_bytes as f64);
    gauges
        .network_received_bytes_per_second
        .set(snapshot.network_received_bytes_per_second);
    gauges
        .network_transmitted_bytes_per_second
        .set(snapshot.network_transmitted_bytes_per_second);
    gauges
        .disk_read_bytes_per_second
        .set(snapshot.disk_read_bytes_per_second);
    gauges
        .disk_written_bytes_per_second
        .set(snapshot.disk_written_bytes_per_second);
    gauges.root_disk_used_bytes.set(
        snapshot
            .root_disk_used_bytes
            .unwrap_or_default() as f64,
    );
    gauges.root_disk_total_bytes.set(
        snapshot
            .root_disk_total_bytes
            .unwrap_or_default() as f64,
    );
}
