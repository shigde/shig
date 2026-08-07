# shig-server

### Architecture

Public traffic:

```text
https://example.com:443        -> nginx stream -> nginx https on 127.0.0.1:8443 -> Angular + /api proxy
https://www.example.com:443    -> nginx stream -> nginx https on 127.0.0.1:8443 -> Angular + /api proxy
https://relay.example.com:443  -> nginx stream -> shig relay on 127.0.0.1:4443
http://*:80                    -> nginx http, certbot challenge, redirect to https
```

Local services:

```text
Shig HTTP API    127.0.0.1:8080
Shig MoQ relay   127.0.0.1:4443
Nginx web HTTPS  127.0.0.1:8443
PostgreSQL       127.0.0.1:5432
```

### Firewall

See [firewall](firewall.md).

Install server packages:

```shell
sudo apt update
sudo apt install -y nginx certbot python3-certbot-nginx postgresql postgresql-client ffmpeg rsync netcat-openbsd
```

Create the runtime user:

```shell
sudo groupadd --system shig
sudo useradd --system --gid shig --home-dir /opt/shig --shell /usr/sbin/nologin shig
```

Create the application directory:

```shell
sudo mkdir -p /opt/shig/bin
sudo mkdir -p /opt/shig/certs/relay
```

Allow the app to create `/opt/shig/htdocs` itself:

```shell
sudo chown shig:shig /opt/shig
sudo chmod 750 /opt/shig
```

Keep config and certificates readable by the `shig` group:

```shell
sudo chown root:shig /opt/shig/config.toml
sudo chmod 640 /opt/shig/config.toml

sudo chown -R root:shig /opt/shig/certs
sudo chmod 750 /opt/shig/certs /opt/shig/certs/relay
sudo chmod 640 /opt/shig/certs/relay/*.pem
```

Give the deployment user access to the binary directory:

```shell
sudo usermod -aG shig thost
sudo chown -R thost:shig /opt/shig/bin
sudo chmod 775 /opt/shig/bin
```

After changing group membership, log out and back in for that user.

### Manual Installation

GitHub Actions normally installs the release binary. For a manual installation, download the release artifact:

```shell
export RELEASE_TAG=<release-tag>
curl -L -o shig_server.tar.gz "https://github.com/shigde/shig/releases/download/${RELEASE_TAG}/shig_server-${RELEASE_TAG}-x86_64-unknown-linux-gnu.tar.gz"
```

Extract the artifact:

```shell
tar -xzf shig_server.tar.gz
```

Install the binary:

```shell
sudo install -m 755 "shig_server-${RELEASE_TAG}-x86_64-unknown-linux-gnu/shig_server" /opt/shig/bin/shig_server
```

Then continue with [config](config.md), certificates, and systemd setup below.

### Config

Create `/opt/shig/config.toml`.

See [config](config.md) for the full production example and field notes.

### Certificates

Get the web certificate:

```shell
sudo certbot --nginx -d example.com -d www.example.com
```

Get the relay certificate:

```shell
sudo certbot certonly --nginx -d relay.example.com
```

Copy the relay certificate for the Shig service user:

```shell
sudo mkdir -p /opt/shig/certs/relay
sudo cp /etc/letsencrypt/live/relay.example.com/fullchain.pem /opt/shig/certs/relay/fullchain.pem
sudo cp /etc/letsencrypt/live/relay.example.com/privkey.pem /opt/shig/certs/relay/privkey.pem
sudo chown -R root:shig /opt/shig/certs
sudo chmod 750 /opt/shig/certs /opt/shig/certs/relay
sudo chmod 640 /opt/shig/certs/relay/*.pem
```

Do not change permissions on `/etc/letsencrypt` if Nginx already uses those files.

See [config](config.md#relay) for the relay certificate paths used by `/opt/shig/config.toml`.

### systemd

Create or edit the service:

```shell
sudo SYSTEMD_EDITOR=nano systemctl edit --full shig.service
```

Use:

```ini
[Unit]
Description=Shig daemon
After=network.target postgresql.service
Wants=postgresql.service

[Service]
Type=simple
User=shig
Group=shig
WorkingDirectory=/opt/shig
ExecStart=/opt/shig/bin/shig_server --config /opt/shig/config.toml
StandardOutput=syslog
StandardError=syslog
SyslogIdentifier=shig
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
```

Apply:

```shell
sudo systemctl daemon-reload
sudo systemctl enable shig.service
sudo systemctl restart shig.service
sudo systemctl status shig.service --no-pager
```

Logs:

```shell
sudo journalctl -u shig.service -n 120 --no-pager
```

### GitHub Actions Deployment

Backend deployment uses GitHub Environment `production`.

Required production secrets:

```text
SERVER_HOST
SERVER_PORT
SERVER_USER
SERVER_TARGET_DIR
SERVER_SSH_KEY
```

Optional production secret:

```text
DB_BACKUP_COMMAND
```

Production vars:

```text
SERVER_SERVICE=shig.service
SERVER_TARGET=x86_64-unknown-linux-gnu
```

Example values:

```text
SERVER_PORT=22
SERVER_TARGET_DIR=/opt/shig
```

The deploy workflow installs only:

```text
/opt/shig/bin/shig_server
```

It does not overwrite:

```text
/opt/shig/config.toml
/opt/shig/certs
/opt/shig/htdocs
```

See [config](config.md) for all values that stay server-local.

The deployment user needs passwordless permission to restart and inspect the service:

```shell
sudo visudo -f /etc/sudoers.d/shig-deploy
```

If `which systemctl` returns `/usr/bin/systemctl`:

```text
thost ALL=(root) NOPASSWD: /usr/bin/systemctl restart shig.service
thost ALL=(root) NOPASSWD: /usr/bin/systemctl status shig.service --no-pager
```

If it returns `/bin/systemctl`, use `/bin/systemctl` instead.

Test:

```shell
sudo -u thost sudo -n /usr/bin/systemctl status shig.service --no-pager
```
