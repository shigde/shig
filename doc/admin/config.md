# Config

This file describes the production `/opt/shig/config.toml`.

Use it together with:

- [shig-server](shig-server.md)
- [web-client](web-client.md)
- [mail](mail.md)
- [postgress](postgress.md)

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

[relay.server]
listen = "127.0.0.1:4443"

[relay.server.tls]
cert = ["/opt/shig/certs/relay/fullchain.pem"]
key = "/opt/shig/certs/relay/privkey.pem"

[relay.web.http]
listen = "127.0.0.1:4444"

[relay.auth]
subscribe = ["live"]
publish = ["live"]
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

This is separate from relay TLS. Relay certificates are configured under `[relay.server.tls]`.

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
```

At the moment there are no production fields to set here.

## Relay

`[relay.*]` configures the Media over QUIC relay.

This is separate from the WebRTC SFU. The SFU handles the conference media, while the relay is used for streaming egress and viewer playback.

`[relay.server]` is the public MoQ/relay listener:

```toml
[relay.server]
listen = "127.0.0.1:4443"
```

With SNI routing, the relay listens locally because Nginx forwards public `relay.example.com:443` to `127.0.0.1:4443`.

`[relay.server.tls]` belongs only to the relay. It is separate from `[server.tls]`.

```toml
[relay.server.tls]
cert = ["/opt/shig/certs/relay/fullchain.pem"]
key = "/opt/shig/certs/relay/privkey.pem"
```

`cert` is a list.

Optional relay HTTP/Web endpoint:

```toml
[relay.web.http]
listen = "127.0.0.1:4444"
```

Relay auth:

```toml
[relay.auth]
subscribe = ["live"]
publish = ["live"]
```
