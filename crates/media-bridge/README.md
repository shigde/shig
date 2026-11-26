# MediaBridge

RTP depacketization and encoded frame assembly for Shig, without a str0m dependency.

Initial scope: H264 and Opus. Selected packet-processing implementations and their
tests will be adapted from str0m to Shig's RTP objects. The existing WebRTC lobby,
GStreamer pipeline, MoQ publishing, and recording paths are unchanged.

This crate currently contains the package scaffold only. No depacketizer or frame
buffer implementation has been imported or connected to the server yet.

## Upstream and licensing

Upstream: <https://github.com/algesten/str0m>, local checkout version 0.26.0.
Reference commit: `b214e10ce22a64b1190c2a089e57a8e67513dae9`.

The upstream MIT license and the Apache-2.0 license accompanying its packet module
are preserved here. When importing code, record the source commit and file paths
and retain any additional file-specific copyright or attribution notices.
