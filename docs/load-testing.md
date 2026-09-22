# Load Testing

The repo ships a load-test harness that doubles as a realistic telemetry
source: `locust-test/` uses [aiolocust](https://aiolocust.io) with
OpenTelemetry exporters pointed at otel-viewer.

## Quickstart

```bash
cd locust-test
uv sync                                   # creates .venv
uv run uvicorn api:app --port 8000        # the system under test
OTEL_METRIC_EXPORT_INTERVAL=5000 uv run aiolocust \
  --host http://localhost:8000 --users 10 --duration 60
```

(The collector must be running first — `cargo run -- --seed-demo` or the
desktop app.)

## What it produces

- traces and logs per request, tagged with the endpoint name
- `locust.current_users` — gauge (stored in `value_int`)
- `locust.client.duration` — **cumulative** histogram
  (`aggregation_temporality = 2`); bounds and bucket_counts are JSON strings,
  cast with `::DOUBLE[]`
- errors and failures as log records with severity ERROR

## Analyzing it

Open the SQL panel and run the built-in examples:

- **Locust: users over time**
- **Locust: latency p95 vs time** — diffs consecutive cumulative snapshots
- **Locust: requests per second** — rate from `hist_count` deltas

See [SQL Console & Charts](sql-console.md) for the charting rules, and
`locust-test/README.md` for the full harness documentation.
