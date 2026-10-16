use anyhow::{anyhow, bail, Context};
use gst::prelude::*;
use gstreamer as gst;
use gstreamer_app::{AppSink, AppSinkCallbacks, AppSrc};
use moq_mux::import::VideoFormat;
use rtc::shared::marshal::Marshal;
use tokio::sync::{mpsc, watch};

const INPUT_MAX_BYTES: u64 = 4 * 1024 * 1024;
const FRAME_QUEUE_SIZE: usize = 128;

#[derive(Clone, Copy, Debug)]
pub(super) enum VideoCodec {
    Vp8,
    Vp9,
    H264,
    H265,
    Av1,
}

impl VideoCodec {
    pub fn from_mime(mime: &str) -> anyhow::Result<Self> {
        match mime.to_ascii_lowercase().as_str() {
            "video/vp8" => Ok(Self::Vp8),
            "video/vp9" => Ok(Self::Vp9),
            "video/h264" => Ok(Self::H264),
            "video/h265" => Ok(Self::H265),
            "video/av1" => Ok(Self::Av1),
            _ => bail!("Unsupported relay video codec: {mime}"),
        }
    }

    pub fn format(self) -> VideoFormat {
        match self {
            Self::Vp8 => VideoFormat::Vp8,
            Self::Vp9 => VideoFormat::Vp9,
            Self::H264 => VideoFormat::Avc3,
            Self::H265 => VideoFormat::Hev1,
            Self::Av1 => VideoFormat::Av01,
        }
    }

    fn elements(self) -> (&'static str, &'static str) {
        match self {
            Self::Vp8 => ("VP8", "rtpvp8depay"),
            Self::Vp9 => ("VP9", "rtpvp9depay"),
            Self::H264 => ("H264", "rtph264depay ! h264parse config-interval=-1 ! video/x-h264,stream-format=byte-stream,alignment=au"),
            Self::H265 => ("H265", "rtph265depay ! h265parse config-interval=-1 ! video/x-h265,stream-format=byte-stream,alignment=au"),
            Self::Av1 => ("AV1", "rtpav1depay ! av1parse ! video/x-av1,stream-format=obu-stream,alignment=tu"),
        }
    }
}

pub(super) struct PipelineConfig {
    pub video_codec: VideoCodec,
    pub video_payload_type: u8,
    pub video_clock_rate: u32,
    pub video_fmtp: String,
    pub audio_payload_type: u8,
    pub audio_channels: u16,
}

pub(super) struct Frame {
    pub video: bool,
    pub sample: gst::Sample,
}

pub(super) struct MediaPipeline {
    pipeline: gst::Pipeline,
    pub video: AppSrc,
    pub audio: AppSrc,
    pub frames: mpsc::Receiver<Frame>,
    pub failure: watch::Receiver<Option<String>>,
}

impl MediaPipeline {
    pub fn start(config: PipelineConfig) -> anyhow::Result<Self> {
        gst::init().context("Initialize GStreamer")?;
        let (encoding, depay) = config.video_codec.elements();
        let description = format!(
            "appsrc name=video_in is-live=true format=time do-timestamp=true block=false \
             ! rtpjitterbuffer latency=50 do-lost=true ! {depay} \
             ! appsink name=video_out sync=false async=false \
             appsrc name=audio_in is-live=true format=time do-timestamp=true block=false \
             ! rtpjitterbuffer latency=50 do-lost=true ! rtpopusdepay \
             ! appsink name=audio_out sync=false async=false"
        );
        let pipeline = gst::parse::launch(&description)?
            .downcast::<gst::Pipeline>()
            .map_err(|_| anyhow!("Expected a GStreamer pipeline"))?;
        let video = pipeline
            .by_name("video_in")
            .unwrap()
            .downcast::<AppSrc>()
            .unwrap();
        let audio = pipeline
            .by_name("audio_in")
            .unwrap()
            .downcast::<AppSrc>()
            .unwrap();
        let mut video_caps = gst::Caps::builder("application/x-rtp")
            .field("media", "video")
            .field("encoding-name", encoding)
            .field("clock-rate", config.video_clock_rate as i32)
            .field("payload", config.video_payload_type as i32)
            .build();
        // RTP fmtp parameters (notably H264 sprop-parameter-sets) belong to the input caps.
        let fields = video_caps.get_mut().unwrap().structure_mut(0).unwrap();
        for field in config.video_fmtp.split(';') {
            if let Some((key, value)) = field.trim().split_once('=') {
                fields.set(key.trim(), value.trim());
            }
        }
        video.set_caps(Some(&video_caps));
        audio.set_caps(Some(
            &gst::Caps::builder("application/x-rtp")
                .field("media", "audio")
                .field("encoding-name", "OPUS")
                .field("clock-rate", 48_000i32)
                .field("payload", config.audio_payload_type as i32)
                .field("encoding-params", config.audio_channels.to_string())
                .build(),
        ));
        video.set_max_bytes(INPUT_MAX_BYTES);
        audio.set_max_bytes(INPUT_MAX_BYTES);
        let (tx, frames) = mpsc::channel(FRAME_QUEUE_SIZE);
        let (failure_tx, failure) = watch::channel(None);
        pipeline.bus().unwrap().set_sync_handler({
            let failure_tx = failure_tx.clone();
            move |_, message| {
                match message.view() {
                    gst::MessageView::Error(err) => {
                        failure_tx.send_replace(Some(format!(
                            "{} ({:?})",
                            err.error(),
                            err.debug()
                        )));
                    }
                    gst::MessageView::Eos(..) => {
                        failure_tx.send_replace(Some("GStreamer reached end of stream".into()));
                    }
                    _ => {}
                }
                // All relevant messages are handled here; do not accumulate a second bus queue.
                gst::BusSyncReply::Drop
            }
        });
        for (name, is_video) in [("video_out", true), ("audio_out", false)] {
            let sink = pipeline
                .by_name(name)
                .unwrap()
                .downcast::<AppSink>()
                .unwrap();
            let tx = tx.clone();
            let failure_tx = failure_tx.clone();
            sink.set_callbacks(
                AppSinkCallbacks::builder()
                    .new_sample(move |sink| {
                        let sample = sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                        tx.try_send(Frame {
                            video: is_video,
                            sample,
                        })
                        .map_err(|err| {
                            failure_tx.send_replace(Some(format!(
                                "Relay frame queue unavailable: {err}"
                            )));
                            gst::FlowError::Error
                        })?;
                        Ok(gst::FlowSuccess::Ok)
                    })
                    .build(),
            );
        }
        let result = Self {
            pipeline,
            video,
            audio,
            frames,
            failure,
        };
        result.pipeline.set_state(gst::State::Playing)?;
        Ok(result)
    }

    pub fn push(source: &AppSrc, packet: &rtp::packet::Packet) -> anyhow::Result<()> {
        if packet.payload.is_empty() {
            return Ok(());
        }
        if source.current_level_bytes() >= INPUT_MAX_BYTES {
            bail!("Relay RTP input queue exceeded {INPUT_MAX_BYTES} bytes");
        }
        // Serialize only the header; the existing Bytes payload is retained by GStreamer.
        // The RTP decoder already removed padding from packet.payload.
        let header = if packet.header.padding {
            let mut header = packet.header.clone();
            header.padding = false;
            header.marshal()?
        } else {
            packet.header.marshal()?
        };
        let mut buffer = gst::Buffer::from_slice(header);
        buffer
            .get_mut()
            .unwrap()
            .append_memory(gst::Memory::from_slice(packet.payload.clone()));
        source.push_buffer(buffer)?;
        Ok(())
    }

    pub fn stop(&self) -> anyhow::Result<()> {
        self.pipeline.set_state(gst::State::Null)?;
        Ok(())
    }
}

impl Drop for MediaPipeline {
    fn drop(&mut self) {
        if let Err(err) = self.stop() {
            log::warn!("Could not stop relay pipeline: {err}");
        }
    }
}
