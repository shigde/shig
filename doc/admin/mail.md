# Mail

## Configuration
Mail is optional. Configure it in `/opt/shig/config.toml`.

See [config](config.md#mail) for the full mail configuration.

If mail is disabled:

```toml
[mail]
enable = false
```

Then signup should not try to send activation emails.

If mail is enabled:

```toml
[mail]
enable = true
smtp_user = "smtp-user@example.com"
smtp_pass = "smtp-password"
smtp_host = "smtp.example.com"
smtp_port = 587
```

## SMTP Setup

If you want to run SMTP on the same Linux server, install Postfix:

```shell
sudo apt update
sudo apt install -y postfix mailutils swaks
```

During setup, choose one of these modes:

- `Internet Site` if the server should send mail directly.
- `Satellite system` if the server should relay mail through an external SMTP provider.

For a simple direct-send setup, set the mail name to your domain, for example:

```text
example.com
```

Check or edit the Postfix config:

```shell
sudo nano /etc/postfix/main.cf
```

Minimal local sending settings:

```conf
myhostname = mail.example.com
myorigin = /etc/mailname
mydestination = localhost
inet_interfaces = loopback-only
inet_protocols = ipv4
```

Restart Postfix:

```shell
sudo systemctl restart postfix
sudo systemctl enable postfix
```

Test local delivery through Postfix:

```shell
echo "Shig SMTP test" | mail -s "Shig SMTP test" target@example.com
```

If Shig should use this local Postfix instance:

```toml
[mail]
enable = true
smtp_user = ""
smtp_pass = ""
smtp_host = "127.0.0.1"
smtp_port = 25
```

See [config](config.md#mail) for the matching `/opt/shig/config.toml` values.

Important: direct mail delivery needs correct DNS records, otherwise many providers will reject or classify messages as spam:

- `A` record for `mail.example.com`
- `MX` record for `example.com`
- `PTR` reverse DNS from your server IP to `mail.example.com`
- `SPF` TXT record
- ideally DKIM and DMARC

For production, using an external SMTP provider on port `587` is often easier than operating direct mail delivery yourself.

## SMTP Test
 
For STARTTLS on port `587`:

```shell
openssl s_client -starttls smtp -connect smtp.example.com:587 -crlf
```

Network check as service user:

```shell
sudo -u shig getent hosts smtp.example.com
sudo -u shig nc -vz smtp.example.com 587
```

For a full SMTP login test:

```shell
sudo apt install -y swaks
swaks --to target@example.com \
  --from smtp-user@example.com \
  --server smtp.example.com \
  --port 587 \
  --auth LOGIN \
  --auth-user 'smtp-user@example.com' \
  --auth-password 'smtp-password' \
  --tls
```

The Linux user `shig` does not need special privileges to send mail. It only needs:

- network access
- DNS resolution
- readable `/opt/shig/config.toml`
- valid SMTP credentials
