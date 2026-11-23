# rtc-interceptor 0.21.1 local patch

Source: the published `rtc-interceptor` 0.21.1 crate from crates.io
(`https://github.com/webrtc-rs/rtc`, MIT/Apache-2.0).
Only this crate is overridden by the application's `[patch.crates-io]` entry;
all other dependencies remain registry dependencies. No registry cache is modified.

## Jitterbuffer discontinuity detection

The original receiver compares the current extended RTP timestamp with its fixed
playout anchor and restarts once their distance exceeds ten seconds. A healthy
continuous stream therefore periodically loses its buffered packets and refills.

The patch compares signed RTP-time progression with elapsed arrival time from the
same anchor. Normal progression no longer triggers a restart, while a divergence
greater than ten seconds retains the existing restart behavior. Depth (120 ms),
capacity (512), interceptor order, and forwarding are unchanged.

The regression test `continuous_stream_survives_multiple_ten_second_boundaries`
drives 30 seconds of virtual media for both 48 kHz and 90 kHz clocks, including
timestamp wrap, and requires every packet at its original playout deadline.
It fails with the original implementation. Existing discontinuity and NACK tests
remain part of validation.

Remove this override when the dependency provides an equivalent upstream fix.
