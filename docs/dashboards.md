# Dashboards

The **Dashboards** view (preset selector in the header) renders YAML-defined
dashboards: a time-range control, filter inputs and a grid of SQL-driven
panels — charts, dials, stats and heatmaps computed from the live DuckDB
database. Think of it as the SQL console with saved, parameterized queries
and proper visualizations.

Three dashboards ship built in:

- **Locust load test** — the classic locust web page rebuilt on raw
  telemetry: requests/s, latency percentiles, user count, a latency-bucket
  heatmap and per-endpoint / per-error request counts. Works with the
  `locust-test/` aiolocust harness.
- **.NET service (auto-instrumentation)** — RED metrics from spans plus the
  ASP.NET Core `RequestDuration` histogram and Kestrel connection gauges.
- **Python service (auto-instrumentation)** — RED metrics from spans plus
  the standard `http.server.duration` histogram and active-requests gauge.

## Where dashboards come from

| Source | Location | Notes |
|---|---|---|
| builtin | embedded in the binary (`src/dashboards/*.yml`) | always available |
| user | `~/.otel-viewer/dashboards/*.yml` / `*.yaml` | rescanned on every request — drop a file in, hit refresh, no restart |

`GET /api/dashboards` lists them; `GET /api/dashboards/{id}` returns the
full document. The sidebar marks user dashboards with a `user` badge.

## Writing a dashboard

```yaml
name: my-service          # -> dashboard id (unique, shown in the URL/API)
title: My Service
description: |
  Shown under the title. Multi-line is fine.

inputs:
  - name: service         # token becomes $service in panel SQL
    label: Service
    type: service         # dropdown of known service names
    default: ""
  - name: route
    label: HTTP route
    type: attribute       # dropdown of distinct values of one attribute
    table: spans          # spans | logs | metrics
    key: http.route       # JSON attribute key on that table
    default: ""
  - name: status
    type: select          # fixed choices
    choices:
      - value: "200"
        label: OK
      - value: "500"
        label: Error

panels:
  - id: rps
    title: Requests / second
    type: line            # line | points | bar | histogram | dial | stat | heatmap
    unit: req/s           # shown as a badge in the panel header
    span: 2               # optional: full width on wide screens (default 1)
    sql: |
      SELECT time_bucket(INTERVAL '30 seconds', to_timestamp(start_ns / 1e9)) AS ts,
             count(*) / 30.0 AS rps
      FROM spans
      WHERE ('$service' IS NULL OR service_name = '$service')
        AND ('$route' IS NULL OR json_extract_string(span_attributes, '$."http.route"') = '$route')
        AND start_ns BETWEEN $from_ns AND $to_ns
      GROUP BY ts ORDER BY ts
```

### Tokens

Panel SQL is a template. Before executing, the UI substitutes:

- `$from_ns` / `$to_ns` — the selected time range as epoch nanoseconds
  ("All time" = `0` … `9223372036854775807`).
- `$<input-name>` — the selected value, **escaped as a SQL string literal**.
  An empty selection substitutes a bare `NULL` (quotes consumed), which is
  why the `('$x' IS NULL OR … = '$x')` idiom turns the filter off.

Everything runs through the same read-only guard as the SQL console
(`src/sqlguard.rs`): single statement, blocklisted keywords, LIMIT appended
when missing. You cannot mutate data from a dashboard.

### Panel types

| type | expects | renders |
|---|---|---|
| `line` | time-like column + numeric columns | multi-line time chart (same detection as the SQL console) |
| `points` | numeric x + numeric y | scatter plot |
| `bar` | label column + numeric columns | vertical bars |
| `histogram` | one numeric column | client-side bucketed histogram |
| `dial` | latest value of first numeric column | radial gauge with auto-scaled max (or set `max:`) |
| `stat` | latest value of first numeric column | big number |
| `heatmap` | time (x) + numeric (y) + numeric (weight) | time × bucket grid, like Grafana's |

Notes for metric panels:

- Histogram bounds/counts are JSON strings — cast with `::DOUBLE[]`.
- `locust.client.duration` (and most OTLP histograms) are **cumulative**:
  diff consecutive snapshots (`cnt - lag(cnt) OVER (PARTITION BY series ORDER BY ts_ns)`)
  before bucketing — see the built-in locust dashboard for the full pattern.
- Histogram bounds may include `+Inf`; filter with `isfinite(b)` before
  using them as axes, and coalesce open-bucket percentiles to the last
  finite bound.
- HUGEINT results (`sum()` over big tables) arrive as strings — the charts
  parse numeric strings, so they still plot.

## Screenshots

See the built-in **Locust load test** dashboard with the `locust-test`
harness running (page: [Load Testing](load-testing.md)).
