use super::pipeline::Frame;
use super::video_recovery::{VideoOutcome, VideoRecovery};
use bytes::Bytes;
use gstreamer as gst;
use moq_mux::import::{Track, VideoFormat, VideoInit};
use std::time::Instant;

#[test]
fn recovery_keeps_keyframe_retries_rate_limited() {
    let now = Instant::now();
    let mut group = super::video_group::VideoGroup::default();
    group.observe(true, 100, now);
    group.reset();
    assert!(group.poll(now));
    group.reset();
    assert!(!group.poll(now + std::time::Duration::from_millis(500)));
    assert!(group.poll(now + std::time::Duration::from_secs(1)));
}

fn frame(key: bool, pts_ms: u64, flags: gst::BufferFlags) -> Frame {
    let mut bytes = vec![0; 32];
    if key {
        bytes[..10].copy_from_slice(&[0x10, 0, 0, 0x9d, 1, 0x2a, 0x40, 1, 0xf0, 0]);
    } else {
        bytes[0] = 0x31;
    }
    let mut buffer = gst::Buffer::from_mut_slice(bytes);
    let buf = buffer.get_mut().unwrap();
    buf.set_pts(gst::ClockTime::from_mseconds(pts_ms));
    buf.set_flags(
        flags
            | if key {
                gst::BufferFlags::empty()
            } else {
                gst::BufferFlags::DELTA_UNIT
            },
    );
    Frame {
        video: true,
        sample: gst::Sample::builder().buffer(&buffer).build(),
    }
}

#[tokio::test]
async fn rewind_recovers_on_valid_keyframe_without_restarting_track() {
    gst::init().unwrap();
    let mut broadcast = moq_net::broadcast::Info::new().produce();
    let catalog =
        moq_mux::catalog::Producer::<()>::new(&mut broadcast, Default::default()).unwrap();
    let mut track = Track::video(
        broadcast.reserve_track("video").unwrap(),
        catalog.reserve(),
        VideoInit::new(VideoFormat::Vp8, Bytes::new()),
    )
    .unwrap();
    let mut recovery = VideoRecovery::default();
    let mut push = |key, pts| {
        recovery
            .publish(
                &mut track,
                frame(key, pts, gst::BufferFlags::empty()),
                "test",
                2,
                64,
                Instant::now(),
            )
            .unwrap()
    };
    assert!(matches!(push(false, 900), VideoOutcome::Dropped));
    assert!(matches!(push(true, 1000), VideoOutcome::Published));
    assert!(matches!(push(false, 1033), VideoOutcome::Published));
    assert!(matches!(push(true, 1100), VideoOutcome::Published));
    assert!(matches!(push(false, 1133), VideoOutcome::Published));
    assert!(matches!(push(false, 16), VideoOutcome::Resync));
    assert!(matches!(push(false, 1166), VideoOutcome::Dropped));
    assert!(matches!(push(true, 1066), VideoOutcome::Resync));
    assert!(matches!(push(true, 1200), VideoOutcome::Published));
    assert!(matches!(push(false, 1233), VideoOutcome::Published));
    track.finish().unwrap();
}

#[tokio::test]
async fn discontinuity_waits_for_keyframe_and_invalid_payload_still_fails() {
    gst::init().unwrap();
    let mut broadcast = moq_net::broadcast::Info::new().produce();
    let catalog =
        moq_mux::catalog::Producer::<()>::new(&mut broadcast, Default::default()).unwrap();
    let mut track = Track::video(
        broadcast.reserve_track("video").unwrap(),
        catalog.reserve(),
        VideoInit::new(VideoFormat::Vp8, Bytes::new()),
    )
    .unwrap();
    let mut recovery = VideoRecovery::default();
    let mut push = |f| recovery.publish(&mut track, f, "test", 0, 0, Instant::now());
    assert!(matches!(
        push(frame(true, 1000, gst::BufferFlags::empty())).unwrap(),
        VideoOutcome::Published
    ));
    assert!(matches!(
        push(frame(false, 1033, gst::BufferFlags::DISCONT)).unwrap(),
        VideoOutcome::Resync
    ));
    assert!(matches!(
        push(frame(false, 1066, gst::BufferFlags::empty())).unwrap(),
        VideoOutcome::Dropped
    ));
    assert!(matches!(
        push(frame(true, 1100, gst::BufferFlags::empty())).unwrap(),
        VideoOutcome::Published
    ));
    let mut invalid = gst::Buffer::from_slice(vec![0]);
    invalid
        .get_mut()
        .unwrap()
        .set_pts(gst::ClockTime::from_mseconds(1200));
    assert!(push(Frame {
        video: true,
        sample: gst::Sample::builder().buffer(&invalid).build()
    })
    .is_err());
    track.finish().unwrap();
}
