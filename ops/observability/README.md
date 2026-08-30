# Shig Observability

This stack starts Prometheus and Grafana for local RTC/SFU metrics.

Start it from this directory:

```bash
docker compose up -d
```

Prometheus scrapes the Shig server at:

```text
https://host.docker.internal:8080/metrics
```

The local Shig setup uses HTTPS with a local certificate, so Prometheus is configured with `insecure_skip_verify: true`.

Grafana is available at:

```text
http://localhost:3000
```

Default login:

```text
admin / admin
```

The pre-provisioned dashboards are named `Shig RTC Overview` and `Shig RTC Diagnostics`.

The dashboard separates two views:

- SFU media router metrics, such as RTP input, forwarded RTP, drops, reconcile activity, and routed PLI/FIR.
- PeerConnection stats exported from `rtc`, such as outbound/inbound bitrate, NACK/PLI/FIR counters, remote inbound RTT/loss, and selected candidate pair RTT. These metrics include a `purpose` label (`participant`, `stream`, or `connection`) so stream video can be diagnosed separately from normal participant video.

`Shig RTC Diagnostics` is for short troubleshooting sessions. Its packet IO panels require:

```toml
[sfu.diagnostics]
packet_io = true
```

Keep this disabled during normal operation.

For a server deployment, change the Prometheus target in `prometheus/prometheus.yml` or proxy `/metrics` through an internal-only Nginx location.
