# Relay Architecture

The stream owner mixes guest and local video/audio in the browser and publishes the composed stream through WebRTC. GStreamer depayloads its RTP packets in the Shig process. `moq-mux` publishes the compressed frames directly into the embedded relay origin. There is no FFmpeg process, local UDP forwarding, fMP4 intermediate, or second QUIC publishing connection.

## Components

```mermaid
flowchart TB
    Client["Browser mixer"] --> Endpoint["Publish Endpoint"]
    Endpoint --> Media["Relay track RTP channels"]
    Actor["RelayActor: owns publisher lifecycle"] -.-> Publisher["HangPublisher: async task"]
    Publisher -.-> Pipeline
    subgraph Pipeline["GStreamer pipeline: native streaming threads"]
        direction LR
        Input["appsrc: audio / video RTP"] --> Jitter["RTP jitter buffers"]
        Jitter --> Depay["Codec depayloaders"]
        Depay --> Output["appsink: compressed frames"]
    end
    Media --> Input
    Output --> Frames["Bounded sample queue"]
    Frames --> Publisher
    Publisher --> Mux["moq-mux: tracks and catalog"]
    Mux --> Origin["Internal MoQ origin"]
    Origin --> Relay["Embedded MoQ relay"]
    Relay --> Player["Viewer player"]
```

`RelayActor` remains the Actix owner. The publisher is an async task, not a second actor. It subscribes to both track channels before starting the pipeline. Startup completes only after the pipeline and MoQ track importers have been created. GStreamer initialization and teardown run on Tokio's blocking pool; GStreamer manages its own streaming threads. The task holds only a weak address to its owning actor.

## Media Path

Video codec, RTP payload type, clock rate, and fmtp parameters come from the received track. The adapter supports VP8, VP9, H264, H265, and AV1 when their GStreamer plugins are installed. H264/H265 parsers output Annex B access units; VP8/VP9 output whole compressed frames. Audio is Opus at a 48 kHz RTP clock, with the negotiated channel count. Unsupported codecs fail explicitly instead of being interpreted as H264. Nothing is decoded, mixed, or re-encoded on the server.

Both paths use a 50 ms RTP jitter buffer. The appsrc arrival timestamps share the pipeline running clock; GStreamer derives frame timestamps from the RTP clocks. This is not full RTCP Sender Report based A/V clock synchronization. Live playback and long-running audio/video synchronization must also be checked with the browser.

The publisher requests a video keyframe after startup. Delta frames are withheld until GStreamer marks a complete keyframe. A discontinuity after startup resets this gate and requests another keyframe. `moq-mux` creates video groups at keyframes; audio groups are explicitly closed after each Opus frame so late viewers do not subscribe to an unbounded audio group.

Browsers are not assumed to generate periodic video keyframes. A check runs every 500 ms, as well as when video frames arrive. The target group duration is two seconds, with 500 ms of initial reaction reserve. The time from the first outstanding PLI to the received keyframe updates this reserve to twice the measured response, bounded between 500 ms and five seconds. Up to one second of that reserve advances the duration trigger; the full reserve is used with the group's average byte/frame growth to predict reaching 75% of moq-net's 32 MiB or 8192-frame limit. An actual keyframe resets the group counters. Unanswered requests, including startup requests without incoming frames, are retried at most once per second. Delta frames never start a new group. These requests reduce the risk of hitting the hard limit; an encoder that ignores PLI or a sudden bitrate burst can still exceed it. Video write errors include the group's frame and payload-byte counts; audio errors are labeled separately.

## Buffer Ownership and Limits

The RTP adapter serializes the header separately and wraps the existing reference-counted payload as GStreamer memory. It does not marshal and copy the entire RTP packet into a new contiguous buffer. Each appsink callback queues an owned sample without blocking. The publisher wraps the readable mapped frame in `Bytes::from_owner`, retaining GStreamer's allocation until MoQ releases it.

This avoids additional payload copies at our Rust/GStreamer boundaries, but is not a claim that depayloading, buffer mapping, parsers, or transport encryption never copy internally.

Each appsrc input queue is limited by a 4 MiB high-water check. The shared output queue holds at most 128 samples. Overflow, RTP broadcast lag, pipeline errors, and closed channels terminate the publication with an error. They do not silently discard arbitrary compressed frames or block the SFU receive loop. These are queue limits, not a strict total pipeline memory budget.

## Shutdown

Stopping a publication cancels the publisher loop, finishes the MoQ tracks, and sets the pipeline to `Null` on the blocking pool. The actor's stop handler waits for task completion. During SFU shutdown the lobby waits for this teardown before replying, so the runtime does not exit first. Dropping the actor also cancels its task. Completion messages identify the running instance so an old shutdown cannot clear a newly started publication. Other broadcasts and the embedded relay remain running.

The embedded `moq_relay::Relay` owns its transport listeners, Web API, and shutdown. `RelayState` provides shared handles for internal publishing and the additional Actix endpoints.

## Visibility

Public streams (`is_public = true`) and their previews are available anonymously. Account-only streams (`false`) and their preview endpoints require a valid login for an active account; no invitation is needed for viewing. Existing publishing and lobby permissions are separate.

The application provides the relay's admission decisions from the database, including the relay's native announcements and fetch routes, so bypassing the Actix API does not bypass visibility. Grants enumerate only accessible broadcast paths and never authorize external publishing. Logged-in players pass their JWT in the relay URL. Authorization runs on connection setup and every 30 seconds for existing leases, not on media packets. The relay serving future owns these lease checks and drops them on shutdown. The web client must be deployed with the backend update for account-only playback.

## Dependencies

Rust uses `gstreamer` and `gstreamer-app` 0.24 with GStreamer >= 1.24. Required runtime elements are `appsrc`, `appsink`, `rtpjitterbuffer`, `rtpopusdepay`, and the selected video depayloader. H264/H265 also require `h264parse`/`h265parse`; AV1 requires `rtpav1depay` and `av1parse`, which must be provided by the installed plugin set. VP8/VP9/Opus do not require an encoder or decoder in production.

On macOS:

```sh
brew install gstreamer
```

On a Debian/Ubuntu release providing GStreamer >= 1.24:

```sh
sudo apt install pkg-config libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad
```

The real-RTP tests additionally use VP8/VP9 and H264 encoders. H264 test generation requires the `x264enc` plugin (`gstreamer1.0-plugins-ugly` on Debian/Ubuntu).

The MoQ libraries are `moq-relay 0.17.5`, `moq-net 0.3.13`, `moq-mux 0.10.14`, `moq-tokio 0.19.26`, and `moq-auth 0.2.1`. Playback uses the existing player's configured latency; the SDK announcement polling remains separate from the player's relay connection.
