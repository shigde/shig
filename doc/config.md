# Config

This file describes the production `/opt/shig/config.toml`.

Use it together with:

- [shig-server](admin/shig-server.md)
- [web-client](admin/web-client.md)
- [mail](admin/mail.md)
- [postgress](admin/postgress.md)

## Example

Create `/opt/shig/config.toml`:

```toml
[server]
host = "127.0.0.1"
port = 8080

[server.tls]
enabled = false
cert = ""
key = ""

[files]
htdocs = "htdocs"

[federation]
enable = true
domain = "example.com"
instance = "shig"
token = "change-this-token"
tls = true

[database]
connection = "postgres://shig:change-this-password@127.0.0.1:5432/shig"
pool_size = 30

[jwt]
auth_token_key = "change-this-auth-token-key"
refresh_token_key = "change-this-refresh-token-key"
session_live_time = 86400

[mail]
enable = false
smtp_user = ""
smtp_pass = ""
smtp_host = ""
smtp_port = 587

[sfu]
advertised_ip = ""
port_min = 50000
port_max = 51000

[relay.listen]
bind = "127.0.0.1:4443"

[relay.listen.tls]
cert = ["/opt/shig/certs/relay/fullchain.pem"]
key = ["/opt/shig/certs/relay/privkey.pem"]

[relay.web.http]
listen = "127.0.0.1:4444"

[relay.auth]
# Shig supplies account-aware embedded authorization.
```

## Webserver

`[server]` configures the Shig HTTP webserver.

This is the server that exposes the REST API, authentication routes, static file access, and the WebRTC signaling endpoints used by the web client.

```toml
[server]
host = "127.0.0.1"
port = 8080
```

When Nginx proxies `/api/` to Shig, keep the HTTP API local.

`[server.tls]` belongs only to this HTTP webserver. It should stay disabled when Nginx terminates HTTPS:

```toml
[server.tls]
enabled = false
cert = ""
key = ""
```

This is separate from relay TLS. Relay certificates are configured under `[relay.listen.tls]`.

## Files

```toml
[files]
htdocs = "htdocs"
```

With `WorkingDirectory=/opt/shig`, this resolves to:

```text
/opt/shig/htdocs
```

If the app should create `htdocs` itself, the runtime user must be able to write to `/opt/shig`.

## Federation

```toml
[federation]
enable = true
domain = "example.com"
instance = "shig"
token = "change-this-token"
tls = true
```

Use your public web domain as `domain`.

## Database

```toml
[database]
connection = "postgres://shig:change-this-password@127.0.0.1:5432/shig"
pool_size = 30
```

If the password contains special URL characters, encode it in the database URL.

Examples:

```text
%  -> %25
@  -> %40
:  -> %3A
/  -> %2F
#  -> %23
?  -> %3F
&  -> %26
+  -> %2B
```

## JWT

```toml
[jwt]
auth_token_key = "change-this-auth-token-key"
refresh_token_key = "change-this-refresh-token-key"
session_live_time = 86400
```

Use long random values for both token keys.

## Mail

Mail is optional.

Disabled mail:

```toml
[mail]
enable = false
smtp_user = ""
smtp_pass = ""
smtp_host = ""
smtp_port = 587
```

With `enable = false`, signup should not send activation emails.

Enabled mail:

```toml
[mail]
enable = true
smtp_user = "smtp-user@example.com"
smtp_pass = "smtp-password"
smtp_host = "smtp.example.com"
smtp_port = 587
```

For a local Postfix instance:

```toml
[mail]
enable = true
smtp_user = ""
smtp_pass = ""
smtp_host = "127.0.0.1"
smtp_port = 25
```

## SFU

`[sfu]` configures the Selective Forwarding Unit.

The SFU handles the live WebRTC conference media between participants. The current config block exists so SFU-specific options can be added without mixing them into the webserver or relay config.

```toml
[sfu]
advertised_ip = ""
port_min = 50000
port_max = 51000
```

## Relay

`[relay.*]` configures the Media over QUIC relay.

This is separate from the WebRTC SFU. The SFU handles the conference media, while the relay is used for streaming egress and viewer playback.

`[relay.listen]` is the public MoQ/relay listener:

```toml
[relay.listen]
bind = "127.0.0.1:4443"
```

With Nginx stream routing, the relay can listen locally because Nginx forwards public relay traffic on `443` to `127.0.0.1:4443`.

TCP `443` uses SNI routing, so `relay.example.com` can share the public HTTPS port with the web client domains. UDP `443` is forwarded completely to the relay for QUIC/WebTransport.

`[relay.listen.tls]` belongs only to the relay. It is separate from `[server.tls]`.

```toml
[relay.listen.tls]
cert = ["/opt/shig/certs/relay/fullchain.pem"]
key = ["/opt/shig/certs/relay/privkey.pem"]
```

`cert` and `key` are lists. Each certificate chain is paired with the key at the same index.

Optional relay HTTP/Web endpoint:

```toml
[relay.web.http]
listen = "127.0.0.1:4444"
```

Relay auth:

```toml
[relay.auth]
# Leave empty: Shig authorizes streams from the database and account login.
```

The relay uses `moq-relay 0.17.x` configuration. When migrating an older configuration, rename `[relay.server]` to `[relay.listen]` and its `listen` field to `bind`. Rename `[relay.client]` to `[relay.connect]`.

Shig installs its own embedded admissions handler. `streams.is_public = true` allows anonymous viewing and preview access; `false` requires a valid login for an active account, not a stream invitation. Publishing, moderation, lobby participation, and editing retain their existing role checks. HTTP preview routes recognize optional Bearer authentication. Anonymous lists omit account-only streams and direct anonymous preview requests return 404.

The MoQ player sends the current account JWT as the `jwt` connection query parameter. Relay grants contain only the accessible broadcasts, with no network publishing permission; the SFU continues publishing internally. Keep relay connections on HTTPS/WebTransport in production and redact JWT query parameters from proxy/access logs. Leases are checked every 30 seconds so expired logins, disabled accounts, and reduced stream visibility close or restrict existing sessions. A token refresh may reconnect playback.

Leave `[relay.auth]` empty. At application startup Shig replaces legacy static public patterns and external auth URLs with this embedded policy and logs a warning, so an older wildcard configuration cannot bypass stream visibility. This is Shig-specific; a standalone `moq-relay` may use different authorization settings.

Static files under `/static`, such as avatar and thumbnail image files, are unchanged by this API/relay policy and remain directly addressable. This policy does not make already-known static image URLs private.
