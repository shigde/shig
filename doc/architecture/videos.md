# Videos and recording resources

A local `RecordedVideo` is a standalone resource, not a livestream or a resumed section of
one. Every recording attempt creates its own UUID. Its `sourceStreamUuid` links
back to the source. Source deletion clears the nullable database foreign key but
preserves the original UUID and the video metadata.

## Metadata and access

Creation copies title, description, support, thumbnail reference, licence,
owner, channel and visibility from the stream. These values are independent
snapshots; editing either resource does not update the other. Livestream-only
settings such as repeat schedules, protocol credentials and stream keys are not
video metadata.

Publication is separate from visibility. New videos have no `publishedAt`.
Only the owner can list, edit, stop or publish their video. Public reads and the
shared preview view expose a video only when it is public, ready and published.
Logging in as someone else does not grant access to private videos. Owners can
read their unpublished/private videos through the owner API.

`PATCH` accepts title, description, support, thumbnail, licence, isPublic and
published. Clients cannot overwrite the owner, source association, duration,
archive location or recording/processing state. Publishing requires a ready video
with an archive reference. Unpublishing preserves the video and its media.

## Lifecycle

`pending -> recording -> processing -> ready`, with `failed` for a recording error.
Only one recording can be active per source stream. Processing an earlier video
or having it published does not prevent a new recording. Stop addresses the video
ID, not the source stream ID. Final duration, archive reference and the ready
transition belong to the future archive-writer integration.

The archive backend remains unconnected. Recording start therefore currently
returns HTTP 503 and stores the attempt as failed, with its error and ID in the
response data. It never claims a successful recording. The metadata-only creation
endpoint creates a pending video without starting an archive writer or touching
the filesystem.

## REST

All paths are relative to `/api`:

| Method | Path | Behavior |
| --- | --- | --- |
| GET | `/videos` | Owner's videos, including private and failed videos |
| POST | `/streams/{streamUuid}/videos` | Create an unpublished metadata snapshot; 201 + Location |
| GET | `/streams/{streamUuid}/videos` | Owner's videos for this source |
| POST | `/streams/{streamUuid}/recordings` | Create a new video and request recording start |
| GET | `/streams/{streamUuid}/recordings/ready` | Wait for actual HLS video initialization; owner only, 204 when ready |
| POST | `/videos/{videoUuid}/stop` | End this recording; repeated stop is idempotent |
| GET | `/videos/{videoUuid}` | Owner metadata and status |
| PATCH | `/videos/{videoUuid}` | Edit metadata, visibility and publication |
| GET | `/pub/videos/{videoUuid}` | Public published video, or authenticated owner preview |

The older singular `/stream/{uuid}/recording` routes remain compatibility stubs;
the SDK now uses the video-resource endpoints above.

## Previews and SDK

`stream_previews` now unions livestreams with public, ready, published videos.
The JSON preview includes `type: "stream" | "video"`, `sourceStreamUuid` and
`durationMs`. Existing public and channel preview endpoints include both types.
Stream visibility behavior is unchanged; private/unpublished videos are excluded.

`recorded_video_previews` is a separate view of just those video rows, using the
same publication/visibility rules. Its public REST endpoints are:

- `GET /api/pub/recorded-video-preview`
- `GET /api/pub/recorded-video-preview/{uuid}`
- `GET /api/pub/channel/{channelUuid}/recorded-video-preview`

The SDK exports `RecordedVideoPreview` through the entity index. `VideoService`
provides `getPreview`, `getPreviewList` and `getChannelPreviewList`, with timestamps
converted to JavaScript Dates. Lists return newest recordings first.

The SDK exports `RecordedVideo`, `RecordedVideoUpdate`, `ContentType` and `VideoService`
through its existing index files. The pre-existing `Video` type is a PeerTube
response and remains unchanged. `MediaService` adapts video resources into the
lobby's compact recording-control state, preserving each video's ID. No automatic
recording polling is enabled.

Video preview cards show Video and duration instead of Offline. They do not poll
relay announcements. `/video/{uuid}` uses the shared metadata/thumbnail page;
livestream guest lookups, lobby controls and live polling are disabled for video.
Actual playback and a dedicated video-management/list UI remain separate work.

## Migration and validation

Migration `2026-10-31-220000_create_videos` adds the table, constraints and union
view; its down migration restores the prior view. The following migration
`2026-10-31-230000_rename_recorded_videos` renames the table to `recorded_videos`
without deleting data. Rust uses `RecordedVideo` / `RecordedVideoUpdate`,
`db::recorded_videos` and `models::user::recorded_video`.
REST paths and the preview discriminator `type: "video"` retain their public names.
No existing recording files are
converted. A PostgreSQL integration test covers snapshots, distinct IDs, owner
access, publication filtering, repeated recordings and source deletion. It requires
`SHIG_MIGRATION_TEST_DATABASE_URL` and runs inside an isolated schema/transaction.
