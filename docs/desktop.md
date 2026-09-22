# Desktop App

The desktop shell (`src-tauri/`, crate `otel-viewer-tauri`) wraps the entire
stack — OTLP receiver, DuckDB, REST router, web UI — in one native window
via Tauri 2. No external services; the collector runs in-process.

## How the window is served: `otelview://`

The webview does **not** load `http://127.0.0.1:6666`. Instead it uses the
custom `otelview://` URI scheme (`https://otelview.localhost/` on Windows),
registered with `register_asynchronous_uri_scheme_protocol` in
`src-tauri/src/lib.rs`. Scheme requests are answered by calling the axum
router **in-process** (`otel_viewer::run_with_db` shares the `Db` handle
with the TCP server).

Why: on macOS 26 (Tahoe), WKWebView silently refuses navigations to plain
`http://127.0.0.1:<port>` — Local Network privacy hardening. No error, no
TCP connection, permanent `about:blank` (a white window). App Transport
Security exceptions in Info.plist do **not** help. The custom scheme makes
the webview's traffic a pure in-process IPC call, which no ATS/Local-Network
policy can block.

The router still binds a real TCP port for **external** use — open
`http://127.0.0.1:6666` in any browser, or curl the API — and the OTLP/gRPC
receiver binds `127.0.0.1:4317` (with fallbacks) for exporters.

## Runtime facts

- Persistent DuckDB at
  - macOS: `~/Library/Application Support/com.otelviewer.desktop/otel-viewer.duckdb`
  - Windows: `%APPDATA%\com.otelviewer.desktop\otel-viewer.duckdb`
- Demo telemetry is seeded when the database is empty
- Single instance: launching a second copy focuses the first
- All listeners bind 127.0.0.1 only

## Build

```bash
pnpm install          # tauri CLI (root package.json)
pnpm -C web install   # web UI dependencies
pnpm tauri build
```

Outputs:

- macOS: `src-tauri/target/release/bundle/macos/otel-viewer.app` and the
  `.dmg` (use `--target universal-apple-darwin` for a universal binary)
- Windows: NSIS installer under `src-tauri/target/release/bundle/nsis/`

Prerequisites and troubleshooting are in `src-tauri/README.md`.
