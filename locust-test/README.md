# locust-test

Tiny example: a FastAPI echo service load-tested with [aiolocust](https://github.com/locustio/aiolocust) (asyncio/aiohttp-based load testing tool).

## Files

- `api.py` — FastAPI app with one endpoint: `POST /api/v1/echo`
  - Request: `{"message": "..."}` → Response: `{"message": "<message> the sum of the first 1000000 is 499999500000"}`
  - the endpoint computes `sum(range(1_000_000))` per request, blocking the
    single asyncio worker — deliberately CPU-heavy to drive latency
    degradation under load
- `locustfile.py` — aiolocust `HttpUser` that posts random messages and verifies the full echo response

## Requirements

aiolocust requires a **freethreaded (no-GIL) Python build** (3.14t). This project pins it via `.python-version` and [uv](https://docs.astral.sh/uv/), which downloads it automatically.

## Run

```bash
# terminal 1 — start the API
uv run uvicorn api:app --port 8000

# terminal 2 — run the load test
uv run aiolocust --host http://localhost:8000 --users 10 --duration 30
```

Example output:

```text
 Name         ┃ Count ┃ Failures ┃    Avg ┃    Max ┃     Rate
━━━━━━━━━━━━━━╇━━━━━━━╇━━━━━━━━━━╇━━━━━━━━╇━━━━━━━━╇━━━━━━━━━━━
 /api/v1/echo │   106 │ 0 (0.0%) │  1.8ms │  8.8ms │   10.59/s
──────────────┼───────┼──────────┼────────┼────────┼────────────
 Total        │   106 │ 0 (0.0%) │  1.8ms │  8.8ms │   10.59/s
```

Useful flags: `--html-report report.html`, `--json-report report.json`, `--iterations N`, `--rate N`.

## Export telemetry to an OTLP collector

aiolocust is OTel-native (`locust.client.duration` histogram, one span per request, etc.) and reads standard `OTEL_*` env vars. Traces, metrics, and logs all default to the `otlp` exporter, but the exporter package matching your collector's protocol must be installed first. For a gRPC collector on port 4317:

```bash
uv add opentelemetry-exporter-otlp-proto-grpc
```

Then point aiolocust at your collector and run the load test:

```bash
OTEL_EXPORTER_OTLP_PROTOCOL=grpc \
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317 \
uv run aiolocust --host http://localhost:8000 --users 10 --duration 30
```

Telemetry is emitted under `service.name=locust`.

- For an HTTP/protobuf collector (port 4318) use `opentelemetry-exporter-otlp-proto-http` with `OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf` and `OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318`
- Per-signal overrides: `OTEL_TRACES_EXPORTER`, `OTEL_METRICS_EXPORTER`, `OTEL_LOGS_EXPORTER` (accept `otlp`, `console`, `none`)
- Quick debug without a collector: `OTEL_TRACES_EXPORTER=console uv run aiolocust ...`

### Plotting users/RPS/latency over time

The OTel SDK's default metric export interval is **60 s**, so short tests emit only one or two points — too coarse to plot ramp-up, RPS, or p95 over time. Lower the interval (milliseconds) and prefer delta temporality so each point is a per-interval histogram delta:

```bash
OTEL_METRIC_EXPORT_INTERVAL=1000 \
OTEL_EXPORTER_OTLP_METRICS_TEMPORALITY_PREFERENCE=delta \
OTEL_EXPORTER_OTLP_PROTOCOL=grpc \
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317 \
uv run aiolocust --host http://localhost:8000 --users 10 --duration 30
```

- `OTEL_METRIC_EXPORT_INTERVAL=1000` → one point per second; the `locust.current_users` gauge also captures ramp-up instead of missing it
- `...TEMPORALITY_PREFERENCE=delta` → each point is a per-interval delta. The default is `cumulative`: every point's `count` is the total since test start, so RPS must be computed as Δcount/Δt between consecutive points — treating cumulative counts as per-second values makes "RPS" grow linearly for the whole test, and dividing by the point's own window makes it decay toward the mean
- Caveat: `OTEL_METRICS_EXPORTER=console` ignores the temporality preference (always cumulative), so don't debug RPS math with the console exporter

Computing RPS and p95 from `locust.client.duration` (unit: seconds):

- Read `aggregation_temporality` from the data instead of assuming: `delta` (1) → RPS per point = Σ `count` / (`time_unix_nano` − `start_time_unix_nano`); `cumulative` (2) → diff consecutive points first
- Sum `count` across **all data points** of an export — failed requests form a separate series carrying an extra `error.type` attribute, so a single series underestimates RPS whenever errors occur
- Use the point's own timestamps, not the configured interval — windows drift slightly (first window is partial, and one extra point is flushed at shutdown)
- p95 comes from `bucket_counts` + `explicit_bounds`; boundaries are 1/2/5/10/20/50 ms, ... so resolution is good once points are frequent

## Notes

- Quick API sanity check:

```bash
curl -X POST http://localhost:8000/api/v1/echo -H "Content-Type: application/json" -d '{"message": "hi"}'
# {"message":"hi the sum of the first 1000000 is 499999500000"}
```
