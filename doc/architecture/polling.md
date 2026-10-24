# Demand-driven batch polling

Authenticated clients use `POST /api/poll`. Only requested, allowlisted resources
are read, sequentially through existing application functions. The endpoint does
not issue HTTP requests to the local application or accept arbitrary URLs.

```json
{"requests":[
  {"id":"counts","resource":"notifications-counts"},
  {"id":"monitor","resource":"monitoring"}
]}
```

A valid batch returns HTTP 200 and `Cache-Control: no-store`, even with partial
failures. Example for a regular account:

```json
{"results":[
  {"id":"counts","status":200,"data":{"total":5,"unread":2}},
  {"id":"monitor","status":403,"error":{"code":"FORBIDDEN","message":"Access to resource denied"}}
]}
```

## Contract

- Request lists must contain 1–32 items with nonempty unique IDs (max 128 bytes).
  Invalid envelopes or JSON return HTTP 400; missing/invalid authentication
  returns HTTP 401 through the existing middleware. An unexpected endpoint-level
  failure may return HTTP 500. Partial failures never turn the batch into an
  HTTP error.
- Allowlisted resources: `messages-counts`, `notifications-counts`, `messages`,
  `notifications`, `monitoring`, `stream-status`, `host-presence`.
- Message/notification identity is always the caller's principal. No recipient
  parameter is accepted. Lists optionally accept integer `offset >= 0`, default
  0, with existing 25-item pagination. Counts accept no parameters.
- Monitoring requires the admin role. Live status accepts `channelUuid` and
  reuses the relay authorization/announcement implementation. Host presence
  accepts `channelUuid` and `streamUuid`, with the existing lobby permission
  checks. Both identifiers must be UUIDs. Unsupported parameters fail per item.
- Stable partial-error codes include `UNKNOWN_RESOURCE`, `INVALID_ARGUMENT`,
  `UNAUTHORIZED`, `FORBIDDEN`, `NOT_FOUND`, `CONFLICT`, `UNAVAILABLE`, `TIMEOUT`,
  and `INTERNAL_ERROR`; errors do not expose internal database details.
- Dispatch has a 2-second per-item async deadline and an 8-second overall budget.
  Inbox reads run on database workers with a 250ms pool acquisition limit and
  1.5-second SQL statement timeouts. Timeouts are returned as partial 504 results.
- Results preserve request order and IDs. Resources do not share a transaction;
  this is a read aggregation endpoint, not an atomic database snapshot.

The SDK scheduler combines due demand into batches and routes each result to the
corresponding subscriber. A partial failure only changes that resource's state
and retry delay. Public anonymous status checks retain the existing public
endpoints; the batch endpoint requires a session.

One-off HTTP read/write endpoints are retained. No database migration is needed;
server, SDK and web client should be released together.
