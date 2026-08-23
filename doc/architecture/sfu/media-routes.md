# SFU Media Routes

This document describes only media-plane routing. Signaling, SDP negotiation, and lobby control messages are intentionally out of scope.

The core rule is that Shig is not a single end-to-end WebRTC connection between publisher and subscriber. It is a set of separate WebRTC legs:

```text
Publisher browser <-> SFU publish endpoint
Subscriber browser <-> SFU subscribe endpoint
```

The SFU may copy media from one leg to another, but RTCP reports, transport feedback, and statistics are scoped to the WebRTC connection on which they are sent. Do not treat publisher-leg RTCP as subscriber-leg RTCP.

## Routing Table

The forwarding table is shaped like the original `webrtc-rs` SFU table, but Shig keeps the logical peer and the split endpoints explicit:

```rust
pub struct ForwardKey {
    pub publisher_peer: PeerId,
    pub publish_endpoint: RtcEndpointId,
    pub mid: Mid,
}

pub struct ForwardTarget {
    pub subscriber_peer: PeerId,
    pub subscribe_endpoint: RtcEndpointId,
    pub sender_id: RTCRtpSenderId,
    pub payload_types: HashMap<u8, u8>,
    pub extension_rewrites: Vec<HeaderExtensionRewrite>,
    pub subscriber_mid: Option<String>,
}

pub struct HeaderExtensionRewrite {
    pub publisher_id: u8,
    pub subscriber_id: u8,
    pub rewrite_mid: bool,
}

pub(crate) struct ForwardTable {
    pub entries: HashMap<ForwardKey, HashMap<PeerId, ForwardTarget>>,
    pub ssrc_index: HashMap<SSRC, ForwardKey>,
}
```

## Reconcile

Reconcile updates the forwarding table from the current lobby state.

There are two main cases:

- A publish endpoint adds a new track. The lobby creates a new `ForwardTable.entries` entry for that track.
- A subscribe endpoint is ready to receive tracks. The lobby adds or refreshes a `ForwardTarget` on an existing forward entry.

The `ForwardTarget` stores the negotiated sender and the translation data needed for RTP forwarding: payload-type mapping, header-extension id mapping, and the subscriber-side `mid`.

## RTP

The lobby uses the RTP SSRC and the forwarding table to find the subscribers:

```text
RTP SSRC -> ForwardKey -> ForwardTarget subscribers
```

That is the RTP media route:

```text
PublishEndpoint RTP -> Subscriber targets
```

## RTCP

RTCP can be sent by publish endpoints and subscribe endpoints. Sender Reports, Receiver Reports, Transport-CC feedback, REMB, and similar feedback messages describe sender/receiver state for one physical WebRTC leg.

Normal RTCP stays inside the endpoint that received it:

```text
Endpoint A RTCP -> Endpoint A
```

The special case is a keyframe request. A subscribe endpoint can ask the original publish endpoint for a new keyframe:

```text
SubscribeEndpoint PLI/FIR -> PublishEndpoint
```

This is intentionally one-way. A publish endpoint does not ask a subscribe endpoint for a keyframe.

To route this request, the SFU uses the media SSRC in the RTCP packet:

```text
RTCP media SSRC -> ForwardKey -> PublishEndpoint
```

This does not require a separate RTCP routing table. Normal RTCP stays inside the endpoint that received it, and PLI/FIR can use the existing `ForwardTable.ssrc_index` to find the original publish endpoint.

## Keyframe Requests

The only default cross-leg RTCP route is the keyframe request:

```text
SubscribeEndpoint PLI/FIR -> SFU -> PublishEndpoint
```

```text
RTCP media SSRC -> ForwardKey -> PublishEndpoint
```

This is intentionally narrow.

Do not cross-forward these as generic media-route messages:

- Sender Reports
- Receiver Reports
- Transport-CC feedback
- REMB
- generic receiver statistics

NACK is also not a default cross-leg route. A subscriber NACK should normally be handled on the subscriber leg by the SFU's own sender/repair buffer. Forwarding NACK upstream would require a deliberate packet-cache, sequence-number, RTX, and payload-type design.
