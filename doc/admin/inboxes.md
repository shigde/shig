# Messages and notifications

Both inboxes live in the existing PostgreSQL database. Startup migration
`2026-10-24-110000_create_inboxes` creates `messages`, `notifications` and the
stream notification transition state. Restart the server to apply it; rebuild
and distribute the SDK and web client together.

- Adding a guest creates a persistent message with the host and stream title.
  The invitation and message commit in one transaction. Repeated additions of
  the same membership do not create duplicate messages. Removing and re-adding
  the guest creates a new invitation/message.
- Successfully starting the SFU stream notifies active, accepted guests.
  The host and inactive accounts are excluded. Retried starts do not notify
  twice; stopping or closing the lobby permits a new start event.
- Read state belongs to the recipient and is independent between the inboxes.
  Opening an inbox does not mark everything read. Opening an entry's stream
  saves that entry as read first. Stream access still uses the existing guards.
- Deleting a stream retains inbox history but removes the stream action.
  Deleting an account cascades its inbox entries.
- The first version uses 15-second polling, not WebSocket/browser push. Messages
  are system messages, not free-form direct chat. Existing invitations are not
  retroactively messaged.

## API

Every endpoint requires a valid active account. Recipient identity always comes
from the authenticated session; clients cannot specify another recipient.
Both `/api/messages` and `/api/notifications` provide:

- `GET ?offset=0`: `{message, data: {items, total, unread}}`, 25 entries per page,
  newest first. Negative offsets are rejected. Dates are ISO 8601 UTC.
- `GET /counts`: `{message, data: {total, unread}}`.
- `PATCH /read`: mark all of the caller's entries read.
- `PATCH /{id}/read`: mark one entry read (idempotent); unknown or other-user IDs
  return 404. Requests never change another recipient's entries.

No arbitrary public send endpoint is exposed. Future server events can use the
`messages` and `notifications` repositories. Shared recipient-scoped reads live
in `inbox.rs`. SDK transport is provided by `MessageService` and
`NotificationService`; the existing diagnostic `MessageService.add/clear` API
is retained. Counts cancel requests and clear cached state on account changes.

## Validation

`cargo test --offline` covers the server suite. Database tests require a dedicated
database via `SHIG_MIGRATION_TEST_DATABASE_URL`; run
`cargo test --offline inbox_events -- --ignored` to check event deduplication,
read isolation, stop/restart, offline lobbies, deletion, and migration rollback.
The SDK and web client have targeted inbox service, header, and inbox-list tests.
