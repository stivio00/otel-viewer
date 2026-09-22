# REST API

All endpoints are served on the HTTP port (6666 by default) under `/api`.
CORS is permissive, so the API can be called from any origin. Timestamps are
returned as ISO strings; HUGEINT values come back as strings (a DuckDB JSON
limitation).

## Health & stats

| Endpoint | Description |
|---|---|
| `GET /api/health` | status, version, db file, OTLP address |
| `GET /api/stats` | row counts per signal, distinct services, time range |
| `GET /api/dbstats` | DuckDB storage stats (see below) |

`GET /api/dbstats` returns:

```json
{
  "db_file": "~/.local/share/otel-viewer.duckdb",
  "file_size_bytes": 12288,
  "database_size": "0 bytes",
  "block_size": 262144,
  "total_blocks": 0,
  "used_blocks": 0,
  "free_blocks": 0,
  "checkpoint_count": null,
  "memory_bytes": 2144256,
  "tables": [
    { "table_name": "spans", "estimated_size": 168,
      "column_count": 23, "index_count": 0 }
  ]
}
```

## Telemetry queries

| Endpoint | Description |
|---|---|
| `GET /api/traces` | trace list; filters `service`, `q`, `start_ns`, `end_ns`, `min_duration_ms`, `errors_only`, `limit`, `offset` |
| `GET /api/traces/{trace_id}` | full span list of one trace |
| `GET /api/logs` | log list; filters `service`, `q`, `severity`, `start_ns`, `end_ns`, `trace_id`, `span_id`, `limit`, `offset` |
| `GET /api/metrics` | metric summaries; filters `q`, `limit` |
| `GET /api/metrics/{name}` | points of one metric; filters `service`, `start_ns`, `end_ns`, `limit` |
| `GET /api/services` | per-service counts and first/last seen |
| `GET /api/schema` | table + column listing |
| `GET /api/dashboards` | list dashboards (builtin + user `~/.otel-viewer/dashboards/`) |
| `GET /api/dashboards/{id}` | full dashboard document (inputs, panels, SQL templates) |

## SQL & maintenance

| Endpoint | Description |
|---|---|
| `POST /api/query` | run read-only SQL (`{"sql": "...", "limit": 500}`) |
| `POST /api/reset` | **delete all telemetry data** (spans, logs, metric points) |

`POST /api/query` enforces the rules of `sqlguard`: a single read-only
statement, blocklisted keywords, `--` comments stripped before scanning, and
an automatic LIMIT when none is given. Histogram bounds/bucket counts are
JSON strings — cast in SQL with `::DOUBLE[]`.

## Examples

```bash
curl -s localhost:6666/api/stats | jq
curl -s 'localhost:6666/api/traces?errors_only=true&limit=5' | jq
curl -s -X POST localhost:6666/api/query \
  -H 'content-type: application/json' \
  -d '{"sql": "select service_name, count(*) n from spans group by 1 order by n desc"}' | jq
```
