# SFU Demuxing

The main endpoint-demuxing idea comes from the `webrtc-rs` Sans-I/O SFU design. Shig adapts that idea to its own lobby, endpoint, and forwarding model.

This document explains how the SFU routes incoming UDP packets to the correct WebRTC endpoint and how media packets are then mapped to individual tracks.

There are two different demuxing steps:

```text
UDP packet -> Endpoint
Endpoint   -> Track
```

The ICE `ufrag` is only used for the first step. It does not identify a media track.

## Endpoint Demuxing

One RTC core owns one UDP socket, but that socket can serve multiple WebRTC endpoints. When a UDP packet arrives, the core first has to decide which endpoint should receive it.

The first routable packet is usually a STUN Binding Request from the client.
STUN carries a `USERNAME` attribute:

```text
USERNAME = local_ufrag ":" remote_ufrag
```

Shig therefore encodes routing information into the local `ufrag` before it is written into the SDP answer or offer:

```text
<encoded-lobby-id>/<rtc-endpoint-id>+<random-ufrag-suffix>
```

When the client sends STUN back to the SFU, the demuxer reads this local ufrag
from the STUN `USERNAME` attribute and decodes:

```text
local_ufrag -> RtcLobbyId + RtcEndpointId
```

After that first STUN packet, the demuxer caches the UDP four-tuple:

```text
local address + remote address + protocol -> RtcLobbyId + RtcEndpointId
```

Later RTP and RTCP packets do not carry the ICE ufrag. They are routed by the
cached four-tuple to the same endpoint.

## Why Not Use The Library Ufrag?

The WebRTC library can generate random ICE credentials by itself. That works
when the peer connection that receives the UDP packet is already known.

Shig's RTC core receives packets for multiple endpoints on the same UDP socket.
Without a recognizable local ufrag, the first STUN packet would only contain a
random value. The SFU would then need another lookup table:

```text
generated_local_ufrag -> RtcLobbyId + RtcEndpointId
```

The current design avoids that extra table by making the local ufrag
self-routing. The random suffix still keeps two endpoints from sharing identical
ICE credentials.

## Track Demuxing

The endpoint demuxer only answers this question:

```text
Which endpoint owns this UDP flow?
```

It does not answer:

```text
Which audio or video track is this packet for?
```

Track demuxing happens inside the endpoint and lobby media router.

Each endpoint is one WebRTC peer connection. A peer connection can have multiple
transceivers, and each transceiver has its own SDP media line and `mid`.

```text
Endpoint
|- Transceiver 0 -> mid "0"
|- Transceiver 1 -> mid "1"
`- Transceiver 2 -> mid "2"
```

For forwarding, Shig identifies a published track with:

```rust
ForwardKey {
    publisher: RtcEndpointId,
    mid: String,
}
```

The endpoint identifies the publishing peer connection. The `mid` identifies the
media line, and therefore the track, inside that peer connection.

## RTP Routing

RTP packets carry an SSRC. They do not carry the ICE ufrag.

The lobby's forwarding table binds publisher SSRCs to forwarding keys:

```text
SSRC -> ForwardKey { publisher: RtcEndpointId, mid }
```

This binding is created from SDP when the SSRC is already visible in the remote
description. If the SSRC is not known from SDP, it is completed later when the
WebRTC stack emits the track-open event with the wire SSRC.

The forwarding path in code is:

```text
RtcLobby::poll_read()
  -> endpoint.poll_read()
  -> RTCMessage::RtpPacket
  -> RtcLobby::forward_rtp(endpoint_id, rtp_packet)
  -> ForwardTable::route_by_ssrc(rtp_packet.header.ssrc)
  -> ForwardKey { publisher: RtcEndpointId, mid }
  -> subscriber RTCRtpSenderId values
  -> translate payload type and RTP header extensions per subscriber
  -> RTCRtpSender::write_rtp(forwarded_rtp)
```

The publisher endpoint from the UDP demuxing step is still checked before
forwarding:

```text
if ForwardKey.publisher != packet_source_endpoint {
    drop packet
}
```

This prevents a packet from one endpoint from being forwarded through an SSRC
binding that belongs to another publisher endpoint.

The full media routing path is:

```text
UDP packet
  -> STUN ufrag or cached four-tuple
  -> RtcEndpointId

RTP packet inside that endpoint
  -> SSRC
  -> ForwardKey { publisher endpoint, mid }
  -> subscriber senders
```

## RTCP Routing

RTCP follows the same endpoint demuxing as RTP: the UDP flow is mapped to an
endpoint by the cached four-tuple.

Inside the lobby, RTCP feedback is interpreted by SSRC:

- publisher RTCP is forwarded to subscribers of that media stream
- subscriber PLI/FIR keyframe requests are routed back to the publisher endpoint

The SSRC-to-`ForwardKey` mapping is what lets the SFU route RTCP feedback to the
correct media line.

## Important Distinction

Do not use ICE ufrag, endpoint ID, SSRC, and MID interchangeably.

| Value | Scope | Answers |
| --- | --- | --- |
| ICE `ufrag` | ICE/STUN connection setup | Which endpoint owns this UDP flow? |
| `RtcEndpointId` | SFU endpoint routing | Which peer connection is this? |
| SSRC | RTP/RTCP stream | Which RTP stream is this packet part of? |
| MID | SDP media line / transceiver | Which track or media line is this? |

The SFU needs both levels:

```text
ufrag / four-tuple -> endpoint
SSRC / MID         -> track inside endpoint
```
