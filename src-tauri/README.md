# otel-viewer desktop app (Tauri)

Wraps the whole otel-viewer stack into one native app: the OTLP/gRPC collector,
DuckDB storage, REST API and web UI all run in-process — no external services.
The window is served through the custom `otelview://` URI scheme, which calls
the REST router in-process (no loopback HTTP from the webview, so macOS
App Transport Security / Local Network privacy never apply).

- OTLP/gRPC receiver on `127.0.0.1:4317` (falls back to 4318-4321, then any free port)
- Window content via the custom `otelview://` scheme (Windows: `https://otelview.localhost/`)
- Web UI + REST API on `127.0.0.1:6666` for external access — browser, curl
  (falls back to 6667-6670, then any free port)
- Persistent DuckDB in the OS app-data dir:
  - macOS: `~/Library/Application Support/com.otelviewer.desktop/otel-viewer.duckdb`
  - Windows: `%APPDATA%\com.otelviewer.desktop\otel-viewer.duckdb`
- Demo telemetry is seeded on first run (empty database)
- Single instance: launching a second copy focuses the first

The actually bound ports are reported by `http://127.0.0.1:6666/api/health`.

## Prerequisites (both platforms)

- Rust via <https://rustup.rs>
- Node.js 20+
- pnpm: `corepack enable` (or `npm i -g pnpm`)

### macOS extra

- Xcode command line tools: `xcode-select --install`

### Windows extra

- Visual Studio 2022 **Build Tools** with the *Desktop development with C++*
  workload (provides the MSVC linker; bundled DuckDB is compiled from source)
- WebView2 runtime — preinstalled on Windows 10/11

## Build

```bash
pnpm install          # tauri CLI (root package.json)
pnpm -C web install   # web UI dependencies
pnpm tauri build
```

`beforeBuildCommand` runs `pnpm -C web build` from the repo root, then cargo
builds the release binary with `web/dist` baked in (rust-embed).

Outputs:

- macOS: `src-tauri/target/release/bundle/macos/otel-viewer.app` and
  `src-tauri/target/release/bundle/dmg/otel-viewer_0.1.0_<arch>.dmg`.
  The arch follows the host; for a universal binary:
  `rustup target add x86_64-apple-darwin aarch64-apple-darwin` then
  `pnpm tauri build --target universal-apple-darwin`
- Windows: NSIS installer under `src-tauri/target/release/bundle/nsis/` plus
  the standalone `src-tauri/target/release/otel-viewer-tauri.exe`
  (no console window opens)

Bundle targets are `["app", "dmg", "nsis"]`; each platform silently skips the
targets that don't apply to it.

## Development

```bash
pnpm tauri dev
```

Debug builds read `web/dist` live from disk, but it is only built when dev
starts — re-run `pnpm -C web build` after web changes. For UI hot-reload,
develop against a running collector instead:

```bash
cargo run -- --seed-demo   # collector: UI/API on :6666, OTLP on :4317
pnpm -C web dev            # vite on :5173, proxies /api to :6666
```

## Icons

Replace `src-tauri/app-icon.png`, then regenerate every format:

```bash
pnpm tauri icon src-tauri/app-icon.png
```

## Sending telemetry

Point any OTLP/gRPC exporter at `localhost:4317`. For a ready-made load test
see `../locust-test/README.md` (aiolocust against this collector).

## Troubleshooting

- **White/empty window on macOS 26 (Tahoe)**: WKWebView there silently blocks
  navigations to plain `http://127.0.0.1:<port>` (Local Network privacy
  hardening) — no error, no TCP connection, permanent `about:blank`. ATS
  exceptions in Info.plist do **not** help. This is why the window must use
  the `otelview://` scheme (`register_asynchronous_uri_scheme_protocol` in
  `src/lib.rs`); do not point it back at the HTTP server. If you see a white
  window, you are running a stale build — quit it (single-instance focuses
  the old copy) and replace it. A blank page can also mean the app was built
  without a real `web/dist`; rebuild with `pnpm tauri build`.
- **Port already in use**: the app walks its port lists (6666-6670, 4317-4321)
  and then asks the OS for any free port — check `/api/health` for the real one.
- **macOS Gatekeeper**: locally built apps are ad-hoc signed and open fine on
  the building machine; binaries distributed to others need signing and
  notarization.
- **Windows firewall**: only 127.0.0.1 is bound, so normally no prompt appears.
