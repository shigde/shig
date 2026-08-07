# web-client

### Static Files

The Angular web client is deployed as static files, for example to:

```text
/var/www/shig-web
```

Nginx serves the static files and proxies `/api/` to the Shig backend on `127.0.0.1:8080`.

### Runtime Config

Production values:

```text
SHIG_API_PREFIX=/api
SHIG_RELAY_SERVICE=https://relay.example.com
```

Because Nginx routes `relay.example.com:443` to Shig relay `127.0.0.1:4443`, the public relay URL should not include `:4443`.

Verify after deployment:

```text
https://example.com/assets/env.js
```

Expected shape:

```js
window.SHIG_ENV = {
  SHIG_API_PREFIX: "/api",
  SHIG_RELAY_SERVICE: "https://relay.example.com"
};
```

### Nginx HTTP Config

The regular Nginx site config belongs in `/etc/nginx/sites-available/shig`.

```nginx
server {
    listen 80;
    server_name example.com www.example.com relay.example.com;

    location /.well-known/acme-challenge/ {
        root /var/www/certbot;
    }

    location / {
        return 301 https://$host$request_uri;
    }
}

server {
    listen 127.0.0.1:8443 ssl http2;
    server_name example.com www.example.com;

    ssl_certificate /etc/letsencrypt/live/example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/example.com/privkey.pem;

    root /var/www/shig-web;
    index index.html;

    location /api/ {
        proxy_pass http://127.0.0.1:8080/;
        proxy_http_version 1.1;

        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto https;
    }

    location / {
        try_files $uri $uri/ /index.html;
    }
}
```

Enable the site:

```shell
sudo ln -s /etc/nginx/sites-available/shig /etc/nginx/sites-enabled/shig
```

If the symlink already exists, check it:

```shell
ls -lah /etc/nginx/sites-enabled/shig
readlink -f /etc/nginx/sites-enabled/shig
```

### Nginx SNI Stream Routing

The `stream` block must be on top level in `/etc/nginx/nginx.conf`, not inside `http {}` and not inside `sites-available/shig` if that file is included from `http`.

Check that Nginx has stream support:

```shell
nginx -V 2>&1 | grep -- --with-stream
```

If stream support is missing:

```shell
sudo apt install -y libnginx-mod-stream
```

Add this outside of the `http {}` block:

```nginx
stream {
    map $ssl_preread_server_name $stream_backend {
        relay.example.com shig_relay;
        default web_https;
    }

    upstream web_https {
        server 127.0.0.1:8443;
    }

    upstream shig_relay {
        server 127.0.0.1:4443;
    }

    server {
        listen 443;
        proxy_pass $stream_backend;
        ssl_preread on;
    }
}
```

How this works:

- Nginx reads the TLS ClientHello SNI name.
- `example.com` and `www.example.com` go to Nginx HTTPS on `127.0.0.1:8443`.
- `relay.example.com` goes to Shig relay on `127.0.0.1:4443`.
- The stream router does not terminate TLS and does not use certificates.
- The backend target provides the certificate.

Certificate ownership:

- `example.com` certificate is used by Nginx on `127.0.0.1:8443`.
- `relay.example.com` certificate is used by Shig relay on `127.0.0.1:4443`.

Test and reload:

```shell
sudo nginx -t
sudo systemctl reload nginx
```

### Firewall

See [firewall](firewall.md).
