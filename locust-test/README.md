# locust-test

Tiny example: a FastAPI echo service load-tested with [aiolocust](https://github.com/locustio/aiolocust) (asyncio/aiohttp-based load testing tool).

## Files

- `api.py` — FastAPI app with one endpoint: `POST /api/v1/echo`
  - Request: `{"message": "..."}` → Response: `{"message": "..."}`
- `locustfile.py` — aiolocust `HttpUser` that posts random messages and verifies the echo

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

## Notes

- Quick API sanity check:

```bash
curl -X POST http://localhost:8000/api/v1/echo -H "Content-Type: application/json" -d '{"message": "hi"}'
# {"message":"hi"}
```
