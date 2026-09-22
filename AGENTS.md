# AGENTS.md

Guidance for AI coding agents working in this repository.

## What this is

otel-viewer is a small OTLP/gRPC collector that stores traces, metrics and
logs in DuckDB and serves a web UI plus a REST query API. It ships in two
forms:

- **CLI/server crate** (`src/`): `otel-viewer` — OTLP/gRPC receiver + HTTP
  (REST API + embedded web UI). Run with `cargo run`, point OTLP exporters
  at it.
- **Desktop shell** (`src-tauri/`): `otel-viewer-tauri` — wraps the library
  crate in a Tauri 2 window. The webview is served through a custom
  `otelview://` URI scheme that proxies to the in-process REST router (no
  loopback HTTP from the webview — see "White window" below). The same
  router also binds 127.0.0.1:6666 for external access (browser, curl).
- **Web UI** (`web/`): React + Vite + Tailwind + recharts SPA. Built into
  `web/dist`, embedded into the binary via rust-embed (`src/api.rs`).

## Layout

| Path | Purpose |
|---|---|
| `src/` | collector library crate: gRPC ingestion (`grpc.rs`), DuckDB layer (`db.rs`), REST API + static UI (`api.rs`), query builders (`queries.rs`), read-only SQL guard (`sqlguard.rs`), demo data (`demo.rs`) |
| `src-tauri/` | Tauri 2 desktop shell (`src/lib.rs`), bundler config (`tauri.conf.json`) |
| `web/src/` | React UI: panels (`components/panels/`), API client (`lib/api.ts`) |
| `locust-test/` | aiolocust load-test harness (uv venv) used to generate realistic telemetry |

## Commands

```bash
cargo check                     # workspace check (root crate)
cargo test                      # unit + integration tests
pnpm -C web build               # tsc + vite build of the web UI (writes web/dist)
pnpm -C web dev                 # vite dev server at :5173, proxies /api -> :6666
cargo run -- --seed-demo        # run collector + API + UI at http://127.0.0.1:6666
pnpm tauri dev                  # desktop shell (debug), embedded web/dist build
pnpm tauri build                # desktop shell (release) -> app/dmg (mac), nsis (win)
pnpm tauri icon src-tauri/app-icon.png   # regenerate bundle icons
```

Run `cargo check`, `cargo test` and `pnpm -C web build` before declaring work
done. There is no rustfmt/clippy config beyond the defaults.

## Important facts

- **`web/dist` must exist before release builds.** rust-embed bakes it in at
  compile time (`#[folder = "web/dist"]` in `src/api.rs`). In debug builds
  rust-embed reads files live from disk; in release it uses the embedded
  copies. `tauri.conf.json` runs `pnpm -C web build` as
  `beforeBuildCommand`/`beforeDevCommand` — those commands execute **from the
  repo root**, not from `web/` or `src-tauri/`, hence the `pnpm -C web` form.
- **Ports**: HTTP 6666 (fallbacks 6667–6670, then OS-assigned), OTLP/gRPC
  4317 (fallbacks 4318–4321). Actual addresses are in `/api/health`.
  The desktop app binds 127.0.0.1 only.
- **CORS is already permissive** (`CorsLayer::very_permissive()` in
  `src/api.rs`), and the UI is normally same-origin with the API, so no CORS
  work is needed for any access pattern (vite dev proxies `/api`,
  browser/CLI hits :6666 directly, the Tauri webview uses the scheme).
- **sqlguard** (`src/sqlguard.rs`): user SQL in the SQL panel must stay a
  single read-only statement; keywords are blocklisted and `--` comments are
  stripped before scanning. Queries without a LIMIT get one appended.
- **Maintenance endpoints**: `POST /api/reset` deletes all telemetry rows
  (serialized through the db writer task); `GET /api/dbstats` returns
  DuckDB storage stats (file size, blocks, per-table sizes, memory) — used
  by the header (i) button.
- **No native JS dialogs in the Tauri webview**: `window.confirm/alert` are
  silent no-ops — use inline two-step confirmation / error states instead
  (see the reset button in `web/src/components/AppHeader.tsx`).
- **aiolocust telemetry quirks** (matters for the SQL panel examples in
  `web/src/components/panels/SqlPanel.tsx`):
  - `locust.current_users` is a gauge stored in `value_int` — use
    `coalesce(value_double, value_int)`.
  - `locust.client.duration` histograms are cumulative
    (`aggregation_temporality = 2`); bounds and bucket_counts are JSON
    strings, cast with `::DOUBLE[]`. Rates/percentiles must diff consecutive
    snapshots.
- **Timestamps** from `/api/query` come back as ISO strings; HUGEINT values
  come back as strings (DuckDB JSON limitation, `src/queries.rs`).
- **Desktop app data** lives at `~/Library/Application Support/com.otelviewer.desktop/otel-viewer.duckdb`
  (macOS; equivalent app-data dir on Windows). Deleting it resets to demo data.

## macOS 26 "white window" — solved, do not reintroduce

On macOS 26 (Tahoe), WKWebView silently refuses navigations to plain
`http://127.0.0.1:<port>` (Local Network privacy hardening): no error, no
TCP connection, permanent `about:blank`. App Transport Security exceptions
in Info.plist do **not** help. Therefore the desktop shell must **not**
point its window at the local HTTP server. It uses the custom
`otelview://` scheme (`register_asynchronous_uri_scheme_protocol` in
`src-tauri/src/lib.rs`) which calls the axum router in-process
(`otel_viewer::run_with_db` shares the `Db`). On Windows the same scheme is
served as `https://otelview.localhost/` (WebView2 convention). External
browser/curl access to `http://127.0.0.1:6666` keeps working.
