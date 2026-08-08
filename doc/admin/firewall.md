# Firewall

Only expose public HTTP/HTTPS:

```shell
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw allow 443/udp
```

Do not expose internal services when they bind to `127.0.0.1`:

```text
8080  Shig HTTP API
8443  Nginx local HTTPS backend
4443  Shig relay behind Nginx stream routing
5432  PostgreSQL
```

TCP `443` is used for HTTPS and WebSocket fallback. UDP `443` is used for QUIC/WebTransport relay traffic.

SSH access depends on your server setup. If UFW is active, allow your SSH port before enabling it:

```shell
sudo ufw allow 22/tcp
sudo ufw enable
sudo ufw status verbose
```
