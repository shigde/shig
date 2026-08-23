# SFU Performance Model

This document is the starting point for performance analysis of the RTC architecture. The
numbers below are estimates, not benchmark results. They make the assumptions explicit so later
load tests can replace the unknown constants with measurements from the real implementation.

An SFU forwards encoded RTP packets instead of decoding and re-encoding the media. Resolution
and codec therefore affect the SFU primarily through bitrate, packet rate, number of simulcast
layers, and forwarding fan-out. The server still performs ICE, DTLS, SRTP authentication and
encryption, RTP/RTCP parsing, header rewriting, congestion feedback, socket I/O, and scheduling.
This is consistent with the IETF description of a Selective Forwarding Middlebox in
[RFC 7667](https://www.rfc-editor.org/rfc/rfc7667.html#section-3.7) and WebRTC's centralized RTP
topologies in [RFC 8834](https://www.rfc-editor.org/rfc/rfc8834.html).

Relay transcoding is excluded from this model. An active FFmpeg encoder can consume much more CPU
than packet forwarding and must be measured as a separate workload.

## Baseline Assumptions

The first example assumes:

- every user publishes one video stream and one audio stream;
- every user subscribes to every other user's streams, without self-loopback;
- video bitrate per publisher: `2.5 Mbit/s`;
- audio bitrate per publisher: `64 kbit/s` with 20 ms packets;
- average video RTP payload: approximately `1,200 bytes`;
- no simulcast, retransmissions, packet loss, recording, or transcoding;
- two WebRTC connections per user: one publish and one subscribe endpoint.

For one publisher this gives:

```text
B = 2.500 + 0.064 = 2.564 Mbit/s

video packet rate ≈ 2,500,000 / (1,200 × 8) ≈ 260 packets/s
audio packet rate ≈ 1 / 0.020                =  50 packets/s
P ≈ 310 RTP packets/s per publisher
```

Protocol headers, RTCP, STUN, DTLS, retransmissions and padding add traffic beyond these payload
figures. Measurements should therefore use packets per second in addition to Mbit/s.

## Lobby With Two Users

Two users create four PeerConnections:

```text
User A publish endpoint   ── audio/video ──▶ RTC core
User A subscribe endpoint ◀─ audio/video ── User B

User B publish endpoint   ── audio/video ──▶ RTC core
User B subscribe endpoint ◀─ audio/video ── User A
```

The RTC core receives two audio/video streams and produces two forwarded audio/video streams:

```text
Ingress             = 2 × 2.564 = 5.128 Mbit/s
Egress              = 2 × 2.564 = 5.128 Mbit/s
Total media handled =              10.256 Mbit/s

Inbound packets     ≈ 2 × 310 =   620 packets/s
Forwarded packets   ≈ 2 × 310 =   620 packets/s
Packet operations   ≈            1,240 packets/s
```

This is a small forwarding workload for a modern server CPU when no transcoding is involved. A
responsible CPU percentage cannot be stated before benchmarking because it depends heavily on
the processor, SRTP cipher implementation, memory allocation, system-call batching, Tokio/Actix
scheduling and packet loss. A reasonable initial hypothesis is a low single-digit percentage of
one modern CPU core, but this is a test hypothesis rather than a capacity guarantee.

## General Scaling Formula

Let:

```text
N = users publishing in one lobby
B = average published audio/video bitrate per user
P = average published packet rate per user
F = number of publisher-to-subscriber forwarding edges
```

The general model is:

```text
Ingress bitrate       = N × B
Egress bitrate        = F × B
Total handled bitrate = (N + F) × B

Packet operations     ≈ (N + F) × P
PeerConnections       = 2 × N
```

For full forwarding to every other user:

```text
F = N × (N - 1)

Total handled bitrate = N² × B
Packet operations     ≈ N² × P
```

CPU and network usage therefore grow approximately quadratically with the number of users in a
single fully connected lobby. This is caused by forwarding fan-out, not by the number of Actor
messages. Selective subscriptions reduce `F` and can change practical scaling substantially.

Using the baseline assumptions:

| Users | PeerConnections | Ingress | Egress | Total handled | Packet operations |
|---:|---:|---:|---:|---:|---:|
| 2 | 4 | 5.1 Mbit/s | 5.1 Mbit/s | 10.3 Mbit/s | ~1,240/s |
| 4 | 8 | 10.3 Mbit/s | 30.8 Mbit/s | 41.0 Mbit/s | ~4,970/s |
| 8 | 16 | 20.5 Mbit/s | 143.6 Mbit/s | 164.1 Mbit/s | ~19,870/s |
| 16 | 32 | 41.0 Mbit/s | 615.4 Mbit/s | 656.4 Mbit/s | ~79,470/s |

At this assumed bitrate, a single lobby approaches one Gbit/s of egress at roughly 21 users,
before Ethernet/IP/UDP/RTP overhead, retransmissions and safety margins. Network capacity may
therefore become the limiting resource before aggregate CPU on some hosts.

## Illustrative CPU Sensitivity

For planning, let `C_packet` be the complete average CPU time for receiving or forwarding one
packet, including the relevant crypto, parsing, routing, allocation and socket work. Then:

```text
CPU cores consumed ≈ packet_operations × C_packet / 1 second
```

The following table is deliberately a sensitivity analysis. The `5`, `15`, and `30 µs` values
are hypothetical costs, not measurements of Shig:

| Users | At 5 µs/op | At 15 µs/op | At 30 µs/op |
|---:|---:|---:|---:|
| 2 | 0.6% core | 1.9% core | 3.7% core |
| 4 | 2.5% core | 7.5% core | 14.9% core |
| 8 | 9.9% core | 29.8% core | 59.6% core |
| 16 | 39.7% core | 119.2% (1.19 cores) | 238.4% (2.38 cores) |

The table shows why measuring per-packet cost matters. It also shows the limitation of assigning
an entire lobby to one core: once that lobby requires more than one CPU core, adding idle cores
to the same pool does not help it.

## Multiple Lobbies and RTC Cores

For a core containing several lobbies, its approximate workload is the sum of each lobby's
forwarding workload:

```text
core packet operations ≈ Σ (N_lobby + F_lobby) × P_lobby
```

With full forwarding:

```text
core packet operations ≈ Σ N_lobby² × P_lobby
```

This distinction matters for scheduling. Two lobbies with two users each are much cheaper than
one lobby with four users:

```text
two 2-user lobbies: 2 × 2² =  8 relative packet units
one 4-user lobby:        4² = 16 relative packet units
```

The current `least_loaded` strategy counts lobbies, not their estimated media load. That is a
useful starting point but will distribute CPU poorly when lobby sizes differ. A later scheduler
should use a weighted load signal such as:

```text
estimated packets/s + estimated outbound bitrate + active endpoint count
```

## Multiple SFU Instances

If one SFU process owns `C` RTC cores and the deployment runs `S` SFU processes, capacity for
many independent lobbies scales approximately with `S × C`, subject to network, memory and load
balancing. Lobby assignment must be sticky: every endpoint of one lobby must reach the same SFU
instance and RTC core for the lobby's lifetime.

This architecture scales well for many small and medium lobbies because they can be distributed
over cores and SFU instances. It does not yet scale one very large lobby across multiple cores or
instances. Supporting that later would require partitioning work inside a lobby, for example by
separating ingress processing from subscriber fan-out or assigning subscriber groups to multiple
cores.

## Factors That Change the Estimate

- **Simulcast:** increases ingress bitrate and packet rate because publishers send multiple
  encodings; egress depends on which layer each subscriber receives.
- **Selective subscriptions:** reduce forwarding edges `F` and are the strongest scaling lever.
- **Packet loss:** adds NACK processing, retransmissions and potentially more keyframes.
- **Small packets:** increase CPU per Mbit/s because packet rate and system-call count rise.
- **SRTP and header rewriting:** make CPU track packets and fan-out, not only bitrate.
- **Relay/FFmpeg:** transcoding is a separate CPU workload and may dominate RTC forwarding.
- **TURN:** relayed client traffic adds network and processing load outside or alongside the SFU.
- **Logging and metrics:** per-packet logging must be disabled in production benchmarks.

## Benchmark Plan

The first benchmark should record:

1. CPU time per `RtcCoreActor` and for the complete process.
2. Ingress/egress Mbit/s and packets/s per core and lobby.
3. Active publish/subscribe endpoints and forwarding edges.
4. Event-loop delay and packet receive-to-send latency, including p95 and p99.
5. Packet loss, retransmissions, NACK, PLI and FIR rates.
6. Memory per endpoint and per forwarded track.

Suggested test matrix:

```text
users per lobby: 2, 4, 8, 16, 24
lobbies per core: 1, 10, 100
cores: 1, 2, 4, 8
video: fixed bitrate, then simulcast
network: clean, then controlled loss and jitter
relay: disabled baseline, then enabled separately
```

The measured results should replace `C_packet` and the assumed bitrates in this document. Those
measurements will also provide the correct input for weighted core assignment and Kubernetes
capacity requests and limits.
