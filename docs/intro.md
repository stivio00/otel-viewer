# otel-viewer

otel-viewer is a small, self-hosted **OTLP/gRPC collector** that stores traces,
metrics and logs in an embedded **DuckDB** database and serves a **web UI** plus
a **REST query API** — all in a single binary. No external services, no
configuration files.

```
OTLP exporters ──gRPC:4317──▶ otel-viewer ──▶ DuckDB (otel-viewer.duckdb)
                                  │
                     HTTP :6666 ──┤── REST API (/api/*)
                                  └── web UI (embedded)
```

## Forms

- **Server (CLI)** — `cargo run`, point any OTLP exporter at `localhost:4317`
  and open `http://localhost:6666`. One binary, in-memory or file-backed
  DuckDB.
- **Desktop app** — the same stack wrapped in a native window (Tauri 2), with
  a persistent database in the OS app-data directory. See [Desktop App](desktop.md).

## Features

- **Traces** — waterfall view, service/error filters, span detail with
  attributes, events and links.
- **Logs** — severity filter, full-text search, trace/span correlation.
- **Metrics** — gauges, sums, histograms (bucket view), exponential
  histograms, summaries (quantiles); auto line charts per series.
- **SQL console** — run read-only SQL directly against the DuckDB tables,
  with automatic time-series charting of the result. See
  [SQL Console & Charts](sql-console.md).
- **Storage info** — file size, block usage and per-table sizes via the (i)
  button in the header.
- **Reset** — one click (well, two — it asks for confirmation) wipes all
  telemetry data.

## Quickstart

```bash
cargo run -- --seed-demo     # UI/API on :6666, OTLP on :4317, demo data
```

Then open <http://localhost:6666>. Point your OTLP/gRPC exporter at
`localhost:4317` (standard OTLP port) — the
[locust load-test harness](load-testing.md) is a ready-made source of
realistic telemetry.

## Ports

| Port | Purpose | Fallbacks |
|---|---|---|
| 6666 | HTTP: web UI + REST API | 6667–6670, then OS-assigned |
| 4317 | OTLP/gRPC receiver | 4318–4321, then OS-assigned |

The actually bound addresses are reported by `GET /api/health`.

## Where to go next

- [Architecture](architecture.md) — how the Rust crate is put together
- [REST API](rest-api.md) — endpoint reference
- [SQL Console & Charts](sql-console.md) — query examples and charting rules
- [Desktop App](desktop.md) — building the Tauri shell
