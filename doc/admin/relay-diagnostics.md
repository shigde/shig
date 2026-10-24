# Relay Diagnostics: UDP Buffers and Timestamp Rewinds

## Production Observation

On 2026-10-18, the Ubuntu deployment successfully started its HTTPS and QUIC relay listeners. Two separate conditions appeared in the logs. Do not treat the UDP buffer warnings as proof of the cause of the publisher failure.

## UDP Socket Buffer Limits

```text
UDP receive buffer is smaller than requested ... wanted=8388608 granted=212992
UDP send buffer is smaller than requested ... wanted=8388608 granted=212992
```

The relay requested 8 MiB buffers, but the reported granted value was approximately 208 KiB. These warnings concern the MoQ UDP sockets. Insufficient buffers can increase packet loss during bursts or scheduling delays; the log alone does not prove packet loss occurred or that buffers caused a timestamp rewind. Raising their limits does not fix invalid media timestamps or increase network bandwidth.

Inspect current host limits:

```shell
sysctl net.core.rmem_max net.core.wmem_max
```

If either limit is below 8388608, raise that limit. Do not reduce an already larger value. For an immediate, non-persistent adjustment:

```shell
sudo sysctl -w net.core.rmem_max=8388608
sudo sysctl -w net.core.wmem_max=8388608
```

These are per-socket upper bounds, not a preallocation of 8 MiB for every socket. Existing sockets are not automatically resized by increasing the limits. Restart Shig during a maintenance window so it requests the buffers again:

```shell
sudo systemctl restart shig.service
sudo journalctl -u shig.service -n 80 --no-pager
```

The restart interrupts active sessions. Confirm the buffer warnings disappear; separately verify streaming and any loss counters. For persistence, create `/etc/sysctl.d/90-shig-udp.conf` with the chosen limits:

```ini
net.core.rmem_max = 8388608
net.core.wmem_max = 8388608
```

Apply only this file with `sudo sysctl -p /etc/sysctl.d/90-shig-udp.conf`. Recheck the effective values after reboot in case other configuration overrides them.

## Fatal Video Timestamp Error

```text
relay publisher failed ... relay video group: frames=1, payload_bytes=22961:
frame timestamp is below the live edge
```

This is the observed publisher-ending error, not merely a tuning warning. MoQ rejected a frame timestamp below the live edge reached by earlier groups. The context counters describe the current video group, not the total frames published during the session.

Current path:

1. GStreamer produces depayloaded frames with presentation timestamps (PTS).
2. `src/sfu/relay/publisher.rs::publish_frame` converts PTS to a MoQ timestamp and calls `Track::decode`.
3. Previously, the decode error propagated and terminated the relay publisher. Video recovery now handles `TimestampRewind` without terminating the publisher.

Track/group producer-drop warnings and cancelled subscriptions appeared during the subsequent teardown. They should not be diagnosed as independent root causes without more evidence.

### Required Follow-up

Recovery is implemented in `src/sfu/relay/video_recovery.rs`, following the media-pad policy in [moq-gst](https://github.com/moq-dev/moq/blob/8f13d08d18e4b77e23779e0fe0c0edf25f34351d/rs/moq-gst/src/sink/pad.rs). On a rewind it closes the current group, drops the rejected frame and waits for a valid keyframe. The existing PLI scheduler requests a replacement keyframe and bounds retries. Delta frames are held back while waiting. Discontinuity/corruption signals also trigger recovery; unrelated decode errors still propagate. Timestamps are not rewritten and mapped payload ownership is unchanged.

Warnings named `relay video timestamp rewind` report stream ID, rejected PTS, last accepted PTS, observed keyframe group start, buffer flags and group/cumulative counters. These observations are not MoQ's internal live edge, which the generic `Track` API does not expose. Warning and recovery messages are each limited to one per five seconds per publisher; cumulative rewind/drop counters retain evidence across suppressed messages. No per-RTP-packet logging is added.

After deploying, inspect recovery alongside other publisher failures:

```shell
sudo journalctl -u shig.service --since "30 minutes ago" --no-pager \
  | grep -E 'relay video timestamp rewind|relay video recovered|relay publisher failed'
```

`relay video recovered` means a subsequent video decode succeeded after waiting; it is not proof of browser playback or of the original cause. Persistent waits still require investigation. This policy handles video; audio decode errors remain fatal as before.

This restores service continuity; it does not establish the underlying cause. Investigate:

- PTS and group-boundary behavior at keyframes, discontinuities, loss and source restarts.
- The exact MoQ live-edge contract and our use of decode and group-cut operations.
- Whether a frame is genuinely stale or a new timestamp epoch needs rebasing.

Regression tests cover rewind recovery, rejected stale keyframes, discarded dependent delta frames, discontinuity and unrelated decode errors. Further production observations are needed to determine the original trigger. Do not clamp every timestamp: presentation-time reordering can be valid, and the recovery follows MoQ's explicit rejection rather than enforcing monotonically increasing PTS itself.

Status: defensive video recovery implemented; underlying timestamp cause remains open. No causal link to the UDP buffer limits has been established. Persistent timestamp resets can still leave the video waiting for a valid keyframe; recovery is not a promise of uninterrupted playback.
