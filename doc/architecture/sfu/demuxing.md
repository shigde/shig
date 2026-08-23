# SFU Demuxing

The main endpoint-demuxing idea comes from the `webrtc-rs` Sans-I/O SFU design. Shig adapts that idea to route many WebRTC endpoints through one RTC core UDP socket.

The demuxer has one job:

```text
incoming transport packet -> RtcEndpointId
```

The later media routing rules are documented in [SFU Media Routes](media-routes.md).

## Step 1: STUN, Ufrag, FourTuple

The first packet that identifies an endpoint is a STUN Binding Request. The STUN packet contains the ICE `ufrag`. Shig encodes the endpoint into the local `ufrag`:

```text
local_ufrag -> RtcLobbyId + RtcEndpointId
```

At the same time, the demuxer reads the packet's transport four-tuple.

The four-tuple is the transport source and destination:

```text
source IP
source port
destination IP
destination port
```

As a Rust shape, it is roughly:

```rust
pub struct FourTuple {
    // Destination address of the local RTC core socket: IP + port.
    pub local_addr: SocketAddr,

    // Source address of the remote peer/client socket: IP + port.
    pub peer_addr: SocketAddr,
}
```

For the current media transport this is UDP, so the full route is:

```text
UDP source IP/port + UDP destination IP/port -> RtcEndpointId
```

The result of the first STUN packet is:

```text
FourTuple -> RtcEndpointId
```

## Step 2: Later Packets

After the four-tuple is known, later packets on the same source/destination route are simple:

```text
incoming packet -> FourTuple -> RtcEndpointId
```

That includes DTLS, RTP, and RTCP packets. The endpoint is clear from the four-tuple.

## Boundary

The demuxer does not decide which media track a packet belongs to. It only decides which RTC endpoint receives the packet.

Track/media routing happens later in the lobby:

```text
Demuxer: incoming packet -> RtcEndpointId
Lobby:   RTP/RTCP SSRC   -> media route
```
