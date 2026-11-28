# MediaBridge

RTP depacketization and encoded frame assembly for Shig, without a str0m dependency.

Initial scope: H264 and Opus. Selected packet-processing implementations and their
tests will be adapted from str0m to Shig's RTP objects. The existing WebRTC lobby,
GStreamer pipeline, MoQ publishing, and recording paths are unchanged.

The H.264 depacketizer is compiled in `src/h264.rs`, with 15 upstream tests.
It handles single NAL units, STAP-A and FU-A, with Annex B output by default
and optional length-prefixed output. It expects ordered payloads supplied by a
frame-aware receive buffer. The Opus depacketizer is compiled in `src/opus.rs`,
with two upstream tests. It passes compressed payload bytes through without
decoding them. Receive buffering and server integration are not implemented yet.

`src/frame.rs` defines the receive-buffer input/output containers using the
server's existing RTP header and RTCP sender report types. Extended sequence
numbers and RTP timestamps must be supplied by the caller; clock synchronization
and counter extension are not implemented yet.

Original str0m files, including their embedded tests, remain unchanged in
`upstream/str0m/src/packet/` as review material. See [UPSTREAM.md](UPSTREAM.md)
for provenance, the extraction status and review order.

Run the active module's tests with `cargo test --offline -p media-bridge` from
the server workspace.

## Upstream and licensing

Upstream: <https://github.com/algesten/str0m>, local checkout version 0.26.0.
Reference commit: `b214e10ce22a64b1190c2a089e57a8e67513dae9`.

The upstream MIT license and the Apache-2.0 license accompanying its packet module
are preserved here. When importing code, record the source commit and file paths
and retain any additional file-specific copyright or attribution notices.
