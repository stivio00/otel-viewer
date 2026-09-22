# Docker

For server deployments there are two images:

- **server** — the `otel-viewer` binary (OTLP/gRPC receiver + REST API +
  embedded web UI) storing telemetry in a DuckDB file on a volume
- **web** — the built React SPA served by nginx, proxying `/api` to the
  server container

Both are wired together by `docker-compose.yml`:

```bash
docker compose up -d --build
```

- UI: `http://localhost:8080` (nginx + SPA, `/api` proxied internally)
- REST API (direct, optional): `http://localhost:6666`
- OTLP/gRPC receiver: `localhost:4317` — point your exporters there
- Data persists in the `otel-data` volume (`/data/otel-viewer.duckdb`
  inside the server container)

To reset all telemetry, either `POST /api/reset` from the UI/API or remove
the volume (`docker compose down -v`).

## Images

```yaml
# server
image: ghcr.io/stivio00/otel-viewer:latest
# web (nginx + SPA)
image: ghcr.io/stivio00/otel-viewer-web:latest
```

The release workflow publishes both to GHCR for every `v*` tag
(`:latest` and `:0.1.1`-style version tags), built for `linux/amd64`.
On Apple Silicon machines use the native desktop app (`.dmg`) or add
`platform: linux/amd64` under the compose services (emulated).

## Building manually

```bash
docker build -f Dockerfile.server -t otel-viewer .
docker build -f Dockerfile.web   -t otel-viewer-web .
```

The server image is a two-stage build: Rust toolchain + cmake/clang compile
the crate (DuckDB's bundled C++ is the slow part), then the binary is copied
into `debian:bookworm-slim` running as a non-root user. The web image builds
the SPA with pnpm on Node 24 and serves `dist/` with nginx.
