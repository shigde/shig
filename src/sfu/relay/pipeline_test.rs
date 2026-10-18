use super::pipeline::{MediaPipeline, PipelineConfig, VideoCodec};
use gst::prelude::*;
use gstreamer as gst;
use gstreamer_app::AppSink;
use rtc::shared::marshal::Unmarshal;
use std::time::Duration;

#[test]
fn video_group_requests_keyframes_and_bounds_retries() {
    use super::video_group::VideoGroup;
    let start = std::time::Instant::now();
    let mut group = VideoGroup::default();
    assert!(!group.observe(true, 1000, start));
    assert!(!group.observe(false, 1000, start + Duration::from_secs(1)));
    assert!(group.observe(false, 1000, start + Duration::from_secs(2)));
    assert!(!group.observe(false, 1000, start + Duration::from_millis(2100)));
    assert!(group.observe(false, 1000, start + Duration::from_secs(3)));
    assert!(!group.observe(true, 1000, start + Duration::from_millis(3100)));
    assert!(!group.observe(false, 1000, start + Duration::from_secs(4)));
}

#[test]
fn large_video_group_requests_keyframe_before_the_library_limit() {
    let start = std::time::Instant::now();
    let mut group = super::video_group::VideoGroup::default();
    assert!(!group.observe(true, 1024, start));
    assert!(group.observe(
        false,
        (moq_net::group::MAX_CACHE_BYTES / 2) as usize,
        start + Duration::from_millis(100)
    ));
}

#[test]
fn missing_first_keyframe_is_retried_without_incoming_frames() {
    let start = std::time::Instant::now();
    let mut group = super::video_group::VideoGroup::default();
    group.requested(start);
    assert!(!group.poll(start + Duration::from_millis(500)));
    assert!(group.poll(start + Duration::from_secs(1)));
    assert!(!group.poll(start + Duration::from_millis(1500)));
    assert!(group.poll(start + Duration::from_secs(2)));
    assert!(!group.observe(true, 1000, start + Duration::from_millis(2100)));
    assert!(!group.poll(start + Duration::from_millis(2500)));
}

#[test]
fn observed_encoder_delay_advances_the_next_request() {
    let start = std::time::Instant::now();
    let mut group = super::video_group::VideoGroup::default();
    group.requested(start);
    group.observe(true, 1000, start + Duration::from_millis(500));
    assert!(!group.poll(start + Duration::from_millis(1400)));
    // A measured 500 ms response reserves twice that time for the next GOP.
    assert!(group.poll(start + Duration::from_millis(1500)));
}

#[tokio::test]
async fn repeated_video_groups_can_publish_more_than_one_group_limit() {
    let mut broadcast = moq_net::broadcast::Info::new().produce();
    let catalog =
        moq_mux::catalog::Producer::<()>::new(&mut broadcast, Default::default()).unwrap();
    let mut import = moq_mux::import::Track::video(
        broadcast.reserve_track("video").unwrap(),
        catalog.reserve(),
        moq_mux::import::VideoInit::new(VideoCodec::Vp8.format(), bytes::Bytes::new()),
    )
    .unwrap();
    let mut key = vec![0; 256 * 1024];
    key[..10].copy_from_slice(&[0x10, 0, 0, 0x9d, 0x01, 0x2a, 0x40, 0x01, 0xf0, 0x00]);
    let key = bytes::Bytes::from(key);
    let mut delta = vec![0; 256 * 1024];
    delta[0] = 0x31;
    let delta = bytes::Bytes::from(delta);
    let start = std::time::Instant::now();
    let mut group = super::video_group::VideoGroup::default();
    let mut keyframe = true;
    let mut requests = 0;
    for index in 0..200 {
        let pts = Duration::from_millis(index * 33);
        let frame = if keyframe { key.clone() } else { delta.clone() };
        let request = group.observe(keyframe, frame.len(), start + pts);
        import
            .decode(
                frame,
                Some(moq_net::Timestamp::from_micros(pts.as_micros() as u64).unwrap()),
            )
            .unwrap();
        // Simulate the encoder answering each PLI with the next keyframe.
        keyframe = request;
        requests += usize::from(request);
    }
    assert!(requests >= 3);
    import.finish().unwrap();
}

fn config(codec: VideoCodec) -> PipelineConfig {
    PipelineConfig {
        video_codec: codec,
        video_payload_type: 96,
        video_clock_rate: 90_000,
        video_fmtp: String::new(),
        audio_payload_type: 111,
        audio_channels: 2,
    }
}

#[test]
fn unsupported_video_codec_is_rejected() {
    assert!(VideoCodec::from_mime("video/unknown").is_err());
    assert!(matches!(
        VideoCodec::from_mime("video/VP9").unwrap(),
        VideoCodec::Vp9
    ));
}

// Generate real encoded RTP locally; no camera, network sockets, or codec fixtures required.
async fn depayload(codec: VideoCodec, encoder: &str, payloader: &str) {
    let mut pipeline = MediaPipeline::start(config(codec)).unwrap();
    let source = gst::parse::launch(&format!(
        "videotestsrc num-buffers=4 ! video/x-raw,width=320,height=240,framerate=30/1 \
         ! {encoder} ! {payloader} pt=96 mtu=800 ! appsink name=rtp sync=false"
    ))
    .unwrap()
    .downcast::<gst::Pipeline>()
    .unwrap();
    let sink = source
        .by_name("rtp")
        .unwrap()
        .downcast::<AppSink>()
        .unwrap();
    source.set_state(gst::State::Playing).unwrap();
    while let Some(sample) = sink.try_pull_sample(gst::ClockTime::from_seconds(5)) {
        let map = sample.buffer().unwrap().map_readable().unwrap();
        let mut raw = map.as_slice();
        let packet = rtp::packet::Packet::unmarshal(&mut raw).unwrap();
        MediaPipeline::push(&pipeline.video, &packet).unwrap();
    }
    source.set_state(gst::State::Null).unwrap();
    let frame = tokio::time::timeout(Duration::from_secs(3), pipeline.frames.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(frame.video);
    let buffer = frame.sample.buffer().unwrap();
    assert!(buffer.pts().is_some());
    assert!(!buffer.flags().contains(gst::BufferFlags::DELTA_UNIT));
    assert!(buffer.size() > 0);
    let expected = buffer.map_readable().unwrap().as_slice().to_vec();
    let mut broadcast = moq_net::broadcast::Info::new().produce();
    let catalog =
        moq_mux::catalog::Producer::<()>::new(&mut broadcast, Default::default()).unwrap();
    let consumer = broadcast.consume();
    let mut import = moq_mux::import::Track::video(
        broadcast.reserve_track("video").unwrap(),
        catalog.reserve(),
        moq_mux::import::VideoInit::new(codec.format(), bytes::Bytes::new()),
    )
    .unwrap();
    let subscriber = consumer
        .track("video")
        .unwrap()
        .subscribe(None)
        .await
        .unwrap();
    super::publisher::publish_frame(&mut import, frame).unwrap();
    import.finish().unwrap();
    let mut output = moq_mux::container::Consumer::new(
        subscriber,
        moq_mux::catalog::hang::Container::Legacy(moq_mux::container::Kind::Data),
    );
    let published = tokio::time::timeout(Duration::from_secs(3), output.read())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(published.keyframe);
    // H264 is converted by moq-mux from Annex B to its advertised AVC container format.
    if !matches!(codec, VideoCodec::H264) {
        assert_eq!(published.payload.as_ref(), expected);
    }
    assert!(catalog.snapshot().video.renditions.contains_key("video"));
    pipeline.stop().unwrap();
}

#[tokio::test]
async fn vp8_rtp_yields_keyframe() {
    depayload(VideoCodec::Vp8, "vp8enc deadline=1", "rtpvp8pay").await;
}

#[tokio::test]
async fn vp9_rtp_yields_keyframe() {
    depayload(VideoCodec::Vp9, "vp9enc deadline=1", "rtpvp9pay").await;
}

#[tokio::test]
async fn h264_rtp_yields_annex_b_access_unit() {
    depayload(
        VideoCodec::H264,
        "x264enc tune=zerolatency",
        "rtph264pay config-interval=-1",
    )
    .await;
}

#[tokio::test]
async fn opus_rtp_yields_compressed_audio() {
    let mut pipeline = MediaPipeline::start(config(VideoCodec::Vp8)).unwrap();
    let packet = rtp::packet::Packet {
        header: rtp::header::Header {
            version: 2,
            payload_type: 111,
            sequence_number: 1,
            timestamp: 48_000,
            ssrc: 42,
            ..Default::default()
        },
        payload: bytes::Bytes::from_static(&[0xf8, 0xff, 0xfe]),
    };
    MediaPipeline::push(&pipeline.audio, &packet).unwrap();
    let frame = tokio::time::timeout(Duration::from_secs(3), pipeline.frames.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(!frame.video);
    assert_eq!(
        frame
            .sample
            .buffer()
            .unwrap()
            .map_readable()
            .unwrap()
            .as_slice(),
        packet.payload.as_ref()
    );
    pipeline.stop().unwrap();
}

fn media(video: bool) -> crate::sfu::media::Media {
    use crate::sfu::media::{Media, MediaPurpose, RtpCodecType};
    use crate::sfu::peer::PeerId;
    use rtc::rtp_transceiver::rtp_sender::RTCRtpCodec;
    let (tx, _) = tokio::sync::broadcast::channel(16);
    Media::new(
        PeerId::new("test"),
        "0".into(),
        "track".into(),
        "stream".into(),
        Default::default(),
        RTCRtpCodec {
            mime_type: if video { "video/VP8" } else { "audio/opus" }.into(),
            clock_rate: if video { 90_000 } else { 48_000 },
            channels: if video { 0 } else { 2 },
            ..Default::default()
        },
        if video {
            RtpCodecType::Video
        } else {
            RtpCodecType::Audio
        },
        42,
        None,
        tx,
        false,
        MediaPurpose::STREAM,
        "test".into(),
        if video { 96 } else { 111 },
    )
}

#[tokio::test]
async fn publisher_delivers_frames_and_can_stop_and_restart() {
    let (origin, driver) = moq_net::origin::Producer::new(Default::default());
    let driver = tokio::spawn(moq_net::time::run(driver));
    let mut announcements = origin.consume().announced();
    for _ in 0..2 {
        let video = media(true);
        let audio = media(false);
        let publisher = super::publisher::HangPublisher::new(
            origin.clone(),
            "test".into(),
            super::RelayMediaStream {
                audio: Some(audio.clone()),
                video: Some(video.clone()),
            },
        )
        .unwrap();
        let cancel = tokio_util::sync::CancellationToken::new();
        let (ready, started) = tokio::sync::oneshot::channel();
        let run = tokio::spawn(publisher.run(cancel.clone(), ready));
        tokio::time::timeout(Duration::from_secs(3), started)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let announced = tokio::time::timeout(Duration::from_secs(3), announcements.next())
            .await
            .unwrap()
            .unwrap();
        assert!(announced.kind.is_active());
        let broadcast = origin.consume().request_broadcast("test").await.unwrap();
        let subscriber = broadcast
            .track("video")
            .unwrap()
            .subscribe(None)
            .await
            .unwrap();
        let keyframe = bytes::Bytes::from_static(&[
            0x10, // VP8 RTP payload descriptor: start of partition zero.
            0x10, 0, 0, 0x9d, 0x01, 0x2a, 0x40, 0x01, 0xf0, 0x00,
        ]);
        video
            .relay_rtp_tx
            .send(std::sync::Arc::new(rtp::packet::Packet {
                header: rtp::header::Header {
                    version: 2,
                    payload_type: 96,
                    marker: true,
                    sequence_number: 1,
                    timestamp: 90_000,
                    ssrc: 42,
                    ..Default::default()
                },
                payload: keyframe,
            }))
            .unwrap();
        let mut output = moq_mux::container::Consumer::new(
            subscriber,
            moq_mux::catalog::hang::Container::Legacy(moq_mux::container::Kind::Data),
        );
        let frame = tokio::time::timeout(Duration::from_secs(3), output.read())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(frame.keyframe);
        assert_eq!(
            frame.payload.as_ref(),
            &[0x10, 0, 0, 0x9d, 0x01, 0x2a, 0x40, 0x01, 0xf0, 0x00]
        );
        cancel.cancel();
        tokio::time::timeout(Duration::from_secs(3), run)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let retracted = tokio::time::timeout(Duration::from_secs(3), announcements.next())
            .await
            .unwrap()
            .unwrap();
        assert!(!retracted.kind.is_active());
    }
    drop(announcements);
    drop(origin);
    tokio::time::timeout(Duration::from_secs(3), driver)
        .await
        .unwrap()
        .unwrap();
}
