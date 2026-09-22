# Architecture

otel-viewer is a cargo workspace-style repo with one library crate, one
desktop shell crate and one web frontend:

```
src/           the `otel-viewer` crate: collector library + CLI server
src-tauri/     the `otel-viewer-tauri` crate: Tauri 2 desktop shell
web/           React + Vite + Tailwind SPA, built into web/dist
locust-test/   aiolocust load-test harness (uv venv)
```

## The Rust crate (`src/`)

| Module | Responsibility |
|---|---|
| `lib.rs` | `run()` / `run_with_db()`: owns the listeners, spawns servers |
| `grpc.rs` | OTLP/gRPC receiver (tonic): traces, metrics, logs services |
| `db.rs` | DuckDB layer: one async writer task + a pool of reader connections |
| `rows.rs` | Flat row types, schema DDL, insert helpers |
| `api.rs` | axum REST router + embedded web UI (rust-embed of `web/dist`) |
| `queries.rs` | Read queries and response DTOs for the REST API |
| `sqlguard.rs` | Read-only guard for user SQL from the SQL panel |
| `convert.rs` | OTLP proto → flat rows |
| `demo.rs` | Demo telemetry seeding |

### Data flow

1. `grpc.rs` receives OTLP export requests, `convert.rs` flattens them into
   `SpanRow` / `LogRow` / `MetricPointRow`.
2. `db.rs` funnels all writes through a **single writer task** (an mpsc
   channel + one connection) so inserts and the `Reset` job serialize; each
   batch is one transaction.
3. REST handlers use pooled **reader connections** via `Db::read()`, with all
   blocking DuckDB work dispatched to tokio's blocking pool.

### Storage

Three tables — `spans`, `log_records`, `metric_points` — defined in
`rows.rs` (`SCHEMA_SQL`). Attributes/events/links/histogram buckets are
stored as JSON strings and parsed on the way out. Timestamps are BIGINT
nanoseconds (`start_ns`, `time_ns`, `ts_ns`).

### The web UI embed

`api.rs` uses `rust-embed` with `#[folder = "web/dist"]`:

- **debug builds** read the files live from disk (edit + refresh),
- **release builds** bake them into the binary.

So `web/dist` must exist before a release build; the Tauri config runs
`pnpm -C web build` as `beforeBuildCommand` to guarantee that.

## The web UI (`web/`)

React 19 + Vite + Tailwind 4. The API client lives in `web/src/lib/api.ts`;
panels (Traces, Logs, Metrics, SQL, Schema) in `web/src/components/panels/`.
Server state via @tanstack/react-query, UI state via zustand, charts via
recharts. See [Web UI](web-ui.md).

## The desktop shell (`src-tauri/`)

Wraps the library crate: opens the persistent DuckDB, starts the collector
in-process and shows the UI through a custom `otelview://` URI scheme that
calls the axum router **in-process** — the webview never makes a loopback
HTTP connection, which sidesteps macOS App Transport Security and the
macOS 26 Local Network privacy controls that silently block such
navigations. Details in [Desktop App](desktop.md).
