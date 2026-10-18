# SFU Performance

## Measured Media Workload

The browser measurement used Chrome with two peers sending the same video content. An additional published stream carried a still image. The WebRTC statistics cover approximately 70 seconds and confirm VP8 video and Opus audio.

For this workload, the release build with hardware AES used approximately **7% server process CPU** on the tested Apple Silicon Mac. The matching Samply recording identifies the ARM64 release executable, and its binary UUID was verified. Browser statistics and the server recording overlap for approximately 34 seconds. The browser rates below describe the full 70-second browser interval, rather than only that overlap.

The following traffic was observed at one browser. "Sent" means browser to SFU; "received" means SFU to browser. Average payload bitrate and packet rate are calculated from counter differences over the recorded interval.

| Direction | Media | Resolution | Frame rate | Average payload bitrate | Average RTP packet rate |
| --- | --- | --- | --- | ---: | ---: |
| Sent | VP8 video | 1280 x 720 | Approximately 30 fps | 2.290 Mbit/s | 258 packets/s |
| Sent | Opus audio | - | - | 28.7 kbit/s | 50 packets/s |
| Received | VP8 video | 1280 x 720 | Approximately 30 fps | 2.284 Mbit/s | 257 packets/s |
| Received | VP8 still-image stream | 1920 x 1080 | Approximately 30 fps | 229.3 kbit/s | 30 packets/s |
| Received | Opus audio track 1 | - | - | 23.6 kbit/s | 50 packets/s |
| Received | Opus audio track 2 | - | - | 14.0 kbit/s | 50 packets/s |

This browser sent approximately 2.318 Mbit/s and received approximately 2.551 Mbit/s of RTP payload. These figures exclude RTP headers, SRTP authentication, UDP/IP headers, RTCP, and other transport traffic. They are not totals for the SFU: the dump contains the publish and subscribe connections of one browser, not the statistics of every peer.

The still-image stream continued to deliver approximately 30 frames per second, but used about one tenth of the received 720p video's payload bitrate. It should therefore not be counted as another equally loaded video stream. The two received audio tracks are observable; their contents cannot be determined from RTP statistics alone.

During the measured interval, incoming packet-loss counters stayed at zero, video NACK counters stayed at zero, and neither received video's dropped-frame counter increased. Each video already had one dropped frame at the beginning of the interval. The local video sender reported no quality limitation and no retransmitted bytes.

## Measurement Context

| Configuration | Workload | Observed server CPU |
| --- | --- | ---: |
| Release build with hardware AES | Two source video/audio pairs plus the low-bitrate still-image stream described above | Approximately 7% |

The 7% reading is a reference observation, not CPU usage per connection or a capacity estimate. The measurement pairs browser traffic statistics with a server recording from the same run, but only one browser's traffic is available. Hardware, bitrate, packet rate, and the number of forwarded tracks affect the result; Linux deployment capacity and larger subscriber counts require their own measurements.

NACK generation, NACK response with a retransmission cache, RTCP sender/receiver reports, and TWCC receiver feedback were enabled during these measurements. RTX use depends on negotiation. An SFU-side send-side bandwidth estimator and congestion-control algorithm were not configured.

Profiling showed that the development build spent much of its work in SRTP encryption, decryption, and HMAC-SHA1 authentication. With release optimization and hardware AES, cryptographic processing became substantially less dominant. UDP sending was the largest individual visible active hotspot in the release profile.

## Release Build

Use a release build for deployment and performance comparisons:

```bash
cargo build --release --package shig_server --bin shig_server
```

For profiling, retain debug symbols without disabling optimization:

```bash
CARGO_PROFILE_RELEASE_DEBUG=true cargo build --release --package shig_server --bin shig_server
```

## Hardware AES

Enable hardware AES when the CPU supports it. Each WebRTC connection uses independent SRTP encryption, so software AES can substantially increase CPU usage as traffic grows.

With the tested RustCrypto `aes 0.8.4` dependency, ARM64 builds require `aes_armv8` to enable the hardware backend. This applies to Apple Silicon and ARM64 Linux:

```bash
RUSTFLAGS="--cfg aes_armv8" cargo build --release --package shig_server --bin shig_server
```

This setting enables runtime detection of ARM AES instructions with a software fallback. Add it to existing `RUSTFLAGS` if other flags are already configured. It must be set during compilation; setting it when launching an existing binary has no effect.

On x86/x86_64 Linux, `aes 0.8.4` detects AES-NI by default, unless software AES is explicitly forced. The ARM flag is not required. Verify the selected crypto backend when changing CPU architecture, dependencies, or crypto provider.

For an IntelliJ Cargo run configuration on ARM64, set `RUSTFLAGS` to `--cfg aes_armv8` in the environment-variable editor and add `--release` to the Cargo command. For profiling, also set `CARGO_PROFILE_RELEASE_DEBUG` to `true`. Enter the values without shell quotes.

## Reading Profiles

Measure process CPU alongside the profile and keep traffic conditions consistent when comparing runs. Flamegraph widths show sample proportions, not necessarily process CPU percentages.

On macOS, Samply also records waiting threads. `kevent` is the runtime's operating-system event-wait function. In the current release recording, approximately 83.7% of main-thread sample rows ended in `kevent`, 3.6% in `sendmsg`, and 1.5% in `recvmsg`. These sample proportions include waiting and are not process CPU percentages. The separately observed server CPU was approximately 7%.
