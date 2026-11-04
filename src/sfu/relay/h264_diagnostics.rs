//! Temporary, opt-in capture of GStreamer's H.264 access units before MoQ import.
use super::pipeline::Frame;
use anyhow::{Context, Result};
use gstreamer as gst;
use rtc::shared::marshal::Marshal;
use std::{
    fs::OpenOptions,
    io::{BufWriter, Write},
    path::Path,
    sync::Arc,
    time::Instant,
};
use tokio::{sync::mpsc, task::JoinHandle};

const MAX_BYTES: u64 = 512 * 1024 * 1024;
const QUEUE_SIZE: usize = 512;

#[derive(Debug)]
enum Capture {
    Frame(gst::Sample),
    Rtp {
        packet: Arc<rtp::packet::Packet>,
        elapsed_us: u64,
    },
}

pub(super) struct H264Diagnostics {
    sender: Option<mpsc::Sender<Capture>>,
    task: JoinHandle<Result<()>>,
    started: Instant,
}

impl H264Diagnostics {
    pub async fn open(directory: &Path, stream: &str) -> Option<Self> {
        let directory = directory.to_owned();
        let stream = stream.to_owned();
        let opened = tokio::task::spawn_blocking(move || {
            // Never create storage directories or derive filesystem paths from stream names.
            anyhow::ensure!(directory.is_dir(), "Diagnostic directory does not exist");
            let prefix = directory.join(format!("h264-{}", uuid::Uuid::new_v4()));
            let raw_path = prefix.with_extension("h264");
            let index_path = prefix.with_extension("jsonl");
            let rtp_path = prefix.with_extension("rtp");
            let raw = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&raw_path)?;
            let index = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&index_path)?;
            let mut index = BufWriter::new(index);
            let rtp = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&rtp_path)?;
            writeln!(
                index,
                "{}",
                serde_json::json!({
                    "event": "start", "stream": stream, "format": "annex-b",
                    "stage": "after-gstreamer-before-recovery-and-moq",
                    "max_bytes": MAX_BYTES,
                    "rtp_format": "concatenated-rtp-packets-indexed-by-offset",
                    "rtp_stage": "before-gstreamer-appsrc",
                    "rtp_padding": "removed-as-in-gstreamer-input",
                })
            )?;
            index.flush()?;
            log::info!(
                "H264 diagnostic capture enabled: stream={}, raw={}, index={}, rtp={}",
                stream,
                raw_path.display(),
                index_path.display(),
                rtp_path.display()
            );
            Ok::<_, anyhow::Error>((BufWriter::new(raw), index, BufWriter::new(rtp)))
        })
        .await;
        let (raw, index, rtp) = match opened {
            Ok(Ok(files)) => files,
            result => {
                log::warn!("Could not enable H264 diagnostics: {result:?}");
                return None;
            }
        };
        let (sender, receiver) = mpsc::channel(QUEUE_SIZE);
        let task = tokio::task::spawn_blocking(move || {
            write_capture(raw, index, rtp, receiver, MAX_BYTES)
        });
        Some(Self {
            sender: Some(sender),
            task,
            started: Instant::now(),
        })
    }

    pub fn capture(&mut self, frame: &Frame) {
        self.send(Capture::Frame(frame.sample.clone()));
    }

    pub fn capture_rtp(&mut self, packet: &Arc<rtp::packet::Packet>) {
        self.send(Capture::Rtp {
            packet: Arc::clone(packet),
            elapsed_us: self.started.elapsed().as_micros().min(u64::MAX as u128) as u64,
        });
    }

    fn send(&mut self, item: Capture) {
        let Some(sender) = &self.sender else { return };
        if let Err(error) = sender.try_send(item) {
            // Stop at a contiguous prefix; never skip frames and create a misleading capture.
            log::warn!("H264 diagnostic capture stopped; no further frames or RTP packets captured: {error}");
            self.sender = None;
        }
    }

    pub async fn finish(self) {
        drop(self.sender);
        match self.task.await {
            Ok(Ok(())) => {}
            result => log::warn!("H264 diagnostic capture failed: {result:?}"),
        }
    }
}

fn write_capture(
    mut raw: impl Write,
    mut index: impl Write,
    mut rtp: impl Write,
    mut frames: mpsc::Receiver<Capture>,
    max_bytes: u64,
) -> Result<()> {
    let mut offset = 0u64;
    let mut number = 0u64;
    let mut rtp_offset = 0u64;
    let mut packets = 0u64;
    let mut reason = "input-closed";
    while let Some(item) = frames.blocking_recv() {
        let sample = match item {
            Capture::Frame(sample) => sample,
            Capture::Rtp { packet, elapsed_us } => {
                // Match MediaPipeline::push: the WebRTC receiver already removed padding.
                let mut header = packet.header.clone();
                header.padding = false;
                let encoded_header = header.marshal()?;
                let size = (encoded_header.len() + packet.payload.len()) as u64;
                if size > max_bytes.saturating_sub(offset + rtp_offset) {
                    reason = "size-limit";
                    log::warn!("H264 diagnostic capture reached its {max_bytes} byte limit");
                    break;
                }
                rtp.write_all(&encoded_header)?;
                rtp.write_all(&packet.payload)?;
                packets += 1;
                writeln!(
                    index,
                    "{}",
                    serde_json::json!({
                        "event": "rtp", "number": packets, "offset": rtp_offset, "size": size,
                        "elapsed_us": elapsed_us, "sequence_number": header.sequence_number,
                        "timestamp": header.timestamp, "ssrc": header.ssrc,
                        "payload_type": header.payload_type, "marker": header.marker,
                        "payload_size": packet.payload.len(),
                    })
                )?;
                rtp_offset += size;
                rtp.flush()?;
                index.flush()?;
                continue;
            }
        };
        let buffer = sample.buffer().context("Diagnostic sample has no buffer")?;
        let mapped = buffer
            .map_readable()
            .context("Cannot map diagnostic H264 frame")?;
        let size = mapped.size() as u64;
        if size > max_bytes.saturating_sub(offset + rtp_offset) {
            reason = "size-limit";
            log::warn!("H264 diagnostic capture reached its {max_bytes} byte limit");
            break;
        }
        raw.write_all(mapped.as_slice())?;
        number += 1;
        writeln!(
            index,
            "{}",
            serde_json::json!({
                "event": "frame", "number": number, "offset": offset, "size": size,
                "pts_us": buffer.pts().map(|pts| pts.useconds()),
                "dts_us": buffer.dts().map(|dts| dts.useconds()),
                "keyframe": !buffer.flags().contains(gst::BufferFlags::DELTA_UNIT),
                "flags": format!("{:?}", buffer.flags()),
            })
        )?;
        offset += size;
        // Keep even an interrupted capture readable up to the last complete indexed frame.
        raw.flush()?;
        index.flush()?;
    }
    writeln!(
        index,
        "{}",
        serde_json::json!({
            "event": "end", "frames": number, "bytes": offset, "reason": reason,
            "rtp_packets": packets, "rtp_bytes": rtp_offset,
        })
    )?;
    raw.flush()?;
    index.flush()?;
    rtp.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_exact_bytes_and_timestamps_without_writing_partial_frames_at_limit() {
        gst::init().unwrap();
        let (sender, receiver) = mpsc::channel(4);
        for (pts, bytes) in [
            (1_000, vec![0, 0, 0, 1, 0x65]),
            (2_000, vec![0, 0, 1, 0x41]),
        ] {
            let mut buffer = gst::Buffer::from_mut_slice(bytes);
            buffer
                .get_mut()
                .unwrap()
                .set_pts(gst::ClockTime::from_useconds(pts));
            sender
                .try_send(Capture::Frame(
                    gst::Sample::builder().buffer(&buffer).build(),
                ))
                .unwrap();
        }
        drop(sender);
        let mut raw = Vec::new();
        let mut index = Vec::new();
        write_capture(&mut raw, &mut index, Vec::new(), receiver, 8).unwrap();
        assert_eq!(raw, [0, 0, 0, 1, 0x65]);
        let rows: Vec<serde_json::Value> = String::from_utf8(index)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(rows[0]["pts_us"], 1_000);
        assert_eq!(rows[0]["offset"], 0);
        assert_eq!(rows[0]["size"], 5);
        assert_eq!(rows[1]["frames"], 1);
        assert_eq!(rows[1]["reason"], "size-limit");
    }

    #[test]
    fn records_packet_bytes_and_original_sequence_numbers_across_wraparound() {
        let (sender, receiver) = mpsc::channel(4);
        for sequence in [u16::MAX, 0] {
            let packet = rtp::packet::Packet {
                header: rtp::header::Header {
                    version: 2,
                    sequence_number: sequence,
                    timestamp: 12345,
                    ssrc: 42,
                    payload_type: 96,
                    marker: sequence == 0,
                    padding: true,
                    ..Default::default()
                },
                payload: bytes::Bytes::from_static(&[0x7c, 0x81, 0x12, 0x34]),
                ..Default::default()
            };
            sender
                .try_send(Capture::Rtp {
                    packet: Arc::new(packet),
                    elapsed_us: 77,
                })
                .unwrap();
        }
        drop(sender);
        let mut data = Vec::new();
        let mut index = Vec::new();
        write_capture(Vec::new(), &mut index, &mut data, receiver, 1024).unwrap();
        let rows: Vec<serde_json::Value> = String::from_utf8(index)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(rows[0]["sequence_number"], 65535);
        assert_eq!(rows[1]["sequence_number"], 0);
        assert_eq!(rows[1]["offset"], 16);
        assert_eq!(rows[1]["marker"], true);
        assert_eq!(rows[2]["rtp_packets"], 2);
        assert_eq!(data.len(), 32);
        assert_eq!(&data[12..16], &[0x7c, 0x81, 0x12, 0x34]);
        assert_eq!(data[0] & 0x20, 0, "padding flag must match GStreamer input");
    }
}
