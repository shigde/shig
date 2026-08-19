# Signaling

This document describes how a Shig client negotiates WebRTC sessions with the SFU.

In this document, `client` means any Shig client implementation: browser, native app, bot, or another WebRTC-capable integration.

The initial connection uses WHIP and WHEP over HTTP. Later media changes are signaled over the Control DataChannel.
The SDP metadata contract used by these flows is documented in
[Signaling Metadata](signaling-metadata.md).

## Roles

```mermaid
flowchart LR
    Client["Client RTCPeerConnection"]
    HTTP["HTTP Signaling: WHIP + WHEP"]
    DC["Control DataChannel: renegotiation messages"]
    SFU["SFU Peer: Receiver + Sender PCs"]

    Client <--> HTTP
    HTTP <--> SFU
    Client <--> DC
    DC <--> SFU
```

WHIP is used for ingress: the client sends local media to the SFU.

WHEP is used for egress: the client receives remote media from the SFU.

The Control DataChannel is used after the initial setup. It carries SFU-driven renegotiation offers and client answers when tracks are added or removed.

## Responsibilities

- The SFU creates a new offer when remote media changes for an already connected client.
- The client answers SFU offers over the Control DataChannel.
- The client serializes incoming DataChannel offers before applying them.
- The SFU ignores stale answers by comparing the answer number with the latest offer number.
- Glare is not expected in the normal SFU signaling path because renegotiation is SFU-owned.

## Initial WHIP / WHEP Flow

```mermaid
sequenceDiagram
    participant Client
    participant SFU

    Client->>SFU: POST /whip - SDP offer with local tracks
    SFU-->>Client: SDP answer
    Client->>Client: setRemoteDescription WHIP answer

    Client->>SFU: POST /whep - request receive offer
    SFU-->>Client: SDP offer with remote tracks
    Client->>Client: setRemoteDescription WHEP offer
    Client->>Client: createAnswer
    Client->>SFU: PATCH /whep - SDP answer
```

After this flow, the client has:

- an ingress peer connection for local media sent to the SFU
- an egress peer connection for remote media received from the SFU
- a Control DataChannel for later signaling messages

## DataChannel Renegotiation Flow

```mermaid
sequenceDiagram
    
    participant JoiningClient
    participant SFU
    box
    participant ExistingClient
    participant WebrtcConnection
    end

    JoiningClient->>SFU: WHIP offer with audio and video tracks
    SFU-->>JoiningClient: WHIP answer

    SFU->>SFU: AddMedia audio
    SFU->>SFU: AddMedia video

    SFU->>ExistingClient: DataChannel OfferMsg 1
    ExistingClient->>WebrtcConnection: enqueue offer 1
    WebrtcConnection->>ExistingClient: setRemoteDescription offer 1
    WebrtcConnection->>ExistingClient: createAnswer
    ExistingClient->>SFU: DataChannel AnswerMsg 1

    SFU->>ExistingClient: DataChannel OfferMsg 2
    ExistingClient->>WebrtcConnection: enqueue offer 2
    WebrtcConnection->>ExistingClient: wait until signalingState is stable
    WebrtcConnection->>ExistingClient: setRemoteDescription offer 2
    WebrtcConnection->>ExistingClient: createAnswer
    ExistingClient->>SFU: DataChannel AnswerMsg 2

    SFU->>SFU: ignore stale answers older than latest offer
```

`ExistingClient` and `WebrtcConnection` are part of the same client implementation.

The client queues incoming offers because audio and video can produce closely spaced `AddMedia` events. The queue prevents overlapping `setRemoteDescription` calls.

## Message Shape

```text
OfferMsg {
  number: u64,
  sdp: string
}

AnswerMsg {
  number: u64,
  sdp: string
}
```

The `number` field binds each answer to the SFU offer that created it.
