# SFU Improvement Notes

This document captures the current performance findings for the SFU media plane. It is a working note, not a final optimization plan.

## Current Finding

The SFU currently appears to be packet-bound. The forwarding table and RTP route lookup are not the dominant cost. The expensive parts are the number of RTP/RTX packets that must be emitted and the SRTP encryption/decryption work required for those packets.

The measured RTP forwarding delay inside the Shig media router is very low:

```text
video forward count: 440601
video forward sum:   0.559752s
average:             ~1.27 microseconds
```

Almost all measured video forward operations completed below `100 microseconds`. This means the route lookup and `write_rtp` call path are not currently the main explanation for frame drops or high CPU usage.

## Problem 1: RTP and RTX Packet Volume

The subscriber legs request and receive a very high amount of RTX traffic. In the latest browser stats, RTX represented roughly `40%` to `45%` of received video packets/bytes on affected subscriber connections.

That has two effects:

- the outgoing packet rate increases significantly;
- the SFU has to encrypt and send many more packets than the original media bitrate suggests.

For example, a `10 Mbit/s` video stream can effectively become much more expensive once retransmissions are added. RTX is not only additional bandwidth; it also increases SRTP work, UDP `sendto` calls, queue pressure, and browser-side packet processing.

The current interpretation is that RTCP/RTX is working, but it is working too hard. The next question is why the subscriber leg needs so many retransmissions.

Important open questions:

- Are packets actually lost, or do they arrive too late for the subscriber jitter buffer?
- Are repeated NACKs causing the same RTP sequence numbers to be retransmitted many times?
- Is the configured max bitrate too aggressive for the current SFU, browser, or network path?
- Does the subscriber sender leg need pacing or queue/backpressure behavior?

## Problem 2: SRTP Encryption and Decryption

The CPU profile shows that a large part of the active RTC core is spent in SRTP and UDP output:

```text
~41% __sendto
~26% kevent
~18% SRTP crypto, AES/SHA1 encrypt/decrypt
```

The hot path contains `rtc_srtp` encryption/decryption and software AES functions such as:

```text
aes::soft::fixslice::aes128_encrypt
sha1::compress::compress
rtc_srtp::context::srtp::encrypt_rtp
rtc_srtp::context::srtp::decrypt_rtp
```

This is expected for an SFU that terminates separate WebRTC legs. The publisher leg and subscriber leg are different SRTP sessions, so forwarded media must be decrypted on ingress and encrypted again on egress.

The open question is whether the current build/runtime uses the best available crypto implementation on the deployment target. On macOS, the profile shows software AES symbols. That may not directly match Linux production behavior, but it is still a useful signal: SRTP cost scales with packet count, fan-out, and retransmissions.

## What Is Probably Not The Main Problem

The current data does not point to the Shig forwarding table or RTP route lookup as the main CPU bottleneck:

```text
RtcLobby::poll_read       low self time
RtcLobby::poll_write      low self time
RTCRtpSender::write_rtp   low self time
translate_rtp             not visible as a major hotspot
```

This does not mean the forwarding path should become careless. It still runs per packet and must stay lean. But the current flamegraph says the larger pressure is packet volume plus SRTP and socket I/O.

## Next Investigation Steps

The next improvements should focus on reducing packet pressure and understanding RTX behavior before changing the forwarding model again.

Suggested diagnostics:

- measure NACK burst size: how many sequence numbers each NACK contains;
- measure repeated RTX for the same media SSRC and RTP sequence number;
- compare original RTP rate with RTX rate per subscriber leg;
- correlate RTX spikes with browser `jitterBufferDelay`, `framesDropped`, and `freezeCount`;
- profile production Linux separately, because crypto and syscall behavior may differ from macOS.

Suggested optimization directions:

- tune sender bitrate so the subscriber leg does not immediately enter heavy NACK/RTX behavior;
- reduce avoidable retransmission storms;
- investigate whether the SRTP crypto backend can use hardware acceleration on the target system;
- keep per-packet Prometheus diagnostics disabled outside short test runs;
- avoid per-packet logging entirely during media tests.

## Current Working Hypothesis

The SFU is not primarily slow at deciding where RTP should go. It is under pressure because it must process and emit too many packets, especially RTX packets, and every emitted packet has SRTP and UDP system-call cost. Reducing unnecessary retransmissions should improve both video stability and CPU usage.
