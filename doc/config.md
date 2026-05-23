## Config

```toml
[server]
bind = "0.0.0.0:443"

[relay]
bind = "0.0.0.0:0:4443"
backend = "quinn"

max_streams = 1000

# multiple versions possible:
version = [
"moq-lite-03",
"moq-transport-16",
]

# optional
quic_lb_nonce = 8

# TLS config
cert = "cert.pem"
key = "key.pem"

# Web config 
web_bind = "127.0.0.1:8080"

# Cluster config
cluster_node = "node-a"

# Auth config (flatten)
auth_token = "secret"
```