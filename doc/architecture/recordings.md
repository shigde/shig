# Stream recordings

For the current video resources, metadata snapshots, REST endpoints and preview
view, see [Videos](videos.md). The control boundary below describes the remaining
legacy recorder stubs; the SDK now uses the video-resource API.

The custom Shig frame writer and reader have been removed. There is no `.frames`
serializer, manifest persistence, per-viewer replay publisher, replay clock, or
recording frame tap in the live publisher anymore. Existing recording files are
not migrated or deleted.

`src/sfu/media.rs` declares the media module. Recording control lives in
`src/sfu/media/media_service.rs`; data objects and storage configuration remain
in `recording.rs` and `config.rs`. There is no `mod.rs`.

## Remaining control boundary

The SDK-facing owner-only status, start, stop and publication endpoints remain.
Ownership checks still use the stream owner from the database. Viewer visibility
checks retain public/account-only stream rules and owner-only unpublished previews.
The live publisher reports its lifecycle to the media service, but sends no frames
into it. Live relay publishing and its stream authorization remain in place.

The upstream archive writer and reader are not connected yet. Start therefore
fails explicitly rather than reporting a successful recording. Stop is idempotent
and returns the empty status. No recording can be published or played back in this
intermediate state. The public availability endpoint returns no recording.
Custom playback allocation/start/stop routes and replay-session grants are removed.

## Storage

The optional `media.directory` setting remains; the default is
`files.htdocs/recordings`. The selected recording root is still created during
project initialization and served by the existing Static setup. The media service
itself performs no filesystem reads or writes. The old custom writer and playback
limits (`max_bytes`, `max_playbacks`) have been removed.

## Next step

Connect the upstream archive writer separately. The proposed reader/prefix routing
is documented in [MoQ archive routing](moq-archive-routing.md); it is not implemented
by this cleanup.
