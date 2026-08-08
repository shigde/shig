# Verification

Backend:

```shell
curl http://127.0.0.1:8080/
sudo journalctl -u shig.service -n 120 --no-pager
```

Web:

```shell
curl -I https://example.com
curl https://example.com/assets/env.js
```

Relay TCP/SNI routing:

```shell
openssl s_client -connect relay.example.com:443 -servername relay.example.com
```

Relay UDP/QUIC routing:

```shell
sudo ss -lunp | grep ':443'
```

Nginx:

```shell
sudo nginx -t
sudo ss -ltnp | grep -E ':80|:443|:8080|:8443|:4443'
sudo ss -lunp | grep -E ':443|:4443'
```

Expected listeners:

```text
0.0.0.0:80 or [::]:80
0.0.0.0:443 or [::]:443     TCP
0.0.0.0:443 or [::]:443     UDP
127.0.0.1:8080
127.0.0.1:8443
127.0.0.1:4443
```
