# SFU Implementation Plan

This document tracks the implementation phases for the RTC architecture described in
[RTC Architecture](rtc.md). It is planning material rather than part of the permanent
architecture contract.

## Phase 1: Define the boundary

- Introduce `ParticipantId`, `EndpointId`, and `ChannelId` as distinct concepts.
- Define RTC commands and events independently of HTTP and DataChannel message formats.

## Phase 2: Add the Sans-I/O runtime pool — in progress

- Add an `RtcPoolActor` and configurable `RtcCoreActor` instances.
- Give every core its own UDP port, timeout driver, and optional OS thread.
- Assign each lobby to one core until the lobby stops.
- Add deterministic tests for packet input, timeout handling, and output draining.

## Phase 3: Implement publish endpoints

- Let the lobby obtain one core assignment, then route endpoint signaling directly through
  `Sfu -> Lobby -> Peer -> RtcCoreActor`.
- Represent every publishing PeerConnection as a distinct endpoint.
- Surface discovered tracks to the lobby subscription router.

## Phase 4: Implement subscribe endpoints

- Create a separate subscribe endpoint for each logical participant.
- Build its initial track selection from the lobby subscription graph.
- Keep HTTP/DataChannel signaling in adapters outside the Sans-I/O core.

## Phase 5: Implement explicit media routing

- Replace `get_medias_without_peer` and broadcast-style `AddMedia` behavior with explicit
  `SubscribeTrack` and `UnsubscribeTrack` commands.
- Keep SSRC/MID lookup, RTP forwarding, and RTCP feedback inside the RTC core.
- Verify participant join, leave, mute, track replacement, and reconnect behavior.

## Phase 6: Complete the RTC architecture

- Route all publish and subscribe traffic through the Sans-I/O media plane.
- Remove superseded media implementations and messages.
- Run browser interoperability, load, shutdown, and reconnect tests.
