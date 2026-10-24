# Certificate Renewal

Certbot renews the originals under `/etc/letsencrypt`, not Shig's copied certificates. A deploy hook copies the renewed relay certificate and key and restarts Shig. It runs only after a successful renewal, not on every renewal check.

See [config](../config.md#relay) for the relay TLS paths expected by Shig.

The deployed `relay.shig.de` setup below uses `/opt/shig/certs/` directly, without a `relay` subdirectory. Other installations must substitute their certificate name and destination paths. In `/opt/shig/config.toml`, these destinations must match:

```toml
[relay.listen.tls]
cert = ["/opt/shig/certs/fullchain.pem"]
key = ["/opt/shig/certs/privkey.pem"]
```

The destination directory must already exist and be accessible to the `shig` service user. Keep the hook root-owned and not writable by the service user.

Create the deploy hook directly in Certbot's deploy directory; no symlink is needed:

```shell
sudo nano /etc/letsencrypt/renewal-hooks/deploy/shig-relay-certs.sh
```

Content:

```shell
#!/bin/sh
set -eu

# Nur auf die Erneuerung des Relay-Zertifikats reagieren.
[ "$RENEWED_LINEAGE" = "/etc/letsencrypt/live/relay.shig.de" ] || exit 0

install -o root -g shig -m 640 \
  "$RENEWED_LINEAGE/fullchain.pem" \
  /opt/shig/certs/fullchain.pem

install -o root -g shig -m 640 \
  "$RENEWED_LINEAGE/privkey.pem" \
  /opt/shig/certs/privkey.pem

systemctl restart shig.service
```

Make executable:

```shell
sudo chown root:root /etc/letsencrypt/renewal-hooks/deploy/shig-relay-certs.sh
sudo chmod 750 /etc/letsencrypt/renewal-hooks/deploy/shig-relay-certs.sh
```

`install` copies the files and sets ownership and permissions; it does not install a package. Restarting Shig disconnects active sessions. The certificate-name guard prevents unrelated web certificate renewals from restarting the SFU.

## Scheduled Renewal

Edit root's existing crontab with `sudo crontab -e`. The existing stop/start-based deployment uses:

```cron
0 2 * * * /bin/sh -c '/bin/systemctl stop nginx && { /usr/bin/certbot renew -q; result=$?; /bin/systemctl start nginx; exit "$result"; }'
```

This checks all Certbot-managed certificates daily at 02:00 in the server's timezone, renewing only those due. Nginx is started again even if Certbot fails. Replace the old entry rather than adding a duplicate. The deploy hook needs no separate cron entry.

Stopping Nginx briefly interrupts web access. This is appropriate when the configured authenticator needs its port, such as standalone HTTP validation. An Nginx or webroot authenticator normally does not require stopping Nginx; retain the method matching the existing renewal configuration.

Verify the saved schedule:

```shell
sudo crontab -l
```

## Verification

Inspect managed certificates and the actual copy used by Shig:

```shell
sudo certbot certificates
sudo openssl x509 -in /opt/shig/certs/fullchain.pem -noout -dates
```

Test renewal validation:

```shell
sudo certbot renew --dry-run
```

A normal dry run does not execute deploy hooks. To test copying the current production certificate and restarting Shig explicitly (this interrupts active sessions):

```shell
sudo env RENEWED_LINEAGE=/etc/letsencrypt/live/relay.shig.de \
  /etc/letsencrypt/renewal-hooks/deploy/shig-relay-certs.sh
sudo systemctl status shig.service --no-pager
```

The hook does not automatically update an already stale copy until the next successful renewal. Use the explicit hook invocation above to synchronize it immediately when the Certbot original is valid.
