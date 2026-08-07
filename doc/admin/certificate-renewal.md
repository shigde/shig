# Certificate Renewal

Certbot renews files under `/etc/letsencrypt`. Because Shig uses copied certificates under `/opt/shig/certs/relay`, copy them after renewal and restart Shig.

See [config](config.md#relay) for the relay TLS paths expected by Shig.

Create a deploy hook:

```shell
sudo nano /etc/letsencrypt/renewal-hooks/deploy/shig-relay-certs.sh
```

Content:

```shell
#!/usr/bin/env sh
set -eu

mkdir -p /opt/shig/certs/relay
cp /etc/letsencrypt/live/relay.example.com/fullchain.pem /opt/shig/certs/relay/fullchain.pem
cp /etc/letsencrypt/live/relay.example.com/privkey.pem /opt/shig/certs/relay/privkey.pem
chown -R root:shig /opt/shig/certs
chmod 750 /opt/shig/certs /opt/shig/certs/relay
chmod 640 /opt/shig/certs/relay/*.pem
systemctl restart shig.service
```

Make executable:

```shell
sudo chmod +x /etc/letsencrypt/renewal-hooks/deploy/shig-relay-certs.sh
```

Test:

```shell
sudo certbot renew --dry-run
```
