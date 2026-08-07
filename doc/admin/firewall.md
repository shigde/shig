# Firewall

Only expose public HTTP/HTTPS:

```shell
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
```

Do not expose internal services when they bind to `127.0.0.1`:

```text
8080  Shig HTTP API
8443  Nginx local HTTPS backend
4443  Shig relay behind Nginx SNI routing
5432  PostgreSQL
```

SSH access depends on your server setup. If UFW is active, allow your SSH port before enabling it:

```shell
sudo ufw allow 22/tcp
sudo ufw enable
sudo ufw status verbose
```
