# SQL Console & Charts

The SQL panel runs **read-only** queries against the live DuckDB database —
the same tables the collector writes. The panel ships with example queries
(load-testing focused); pick one from the dropdown to get started.

## Guard rules

- exactly one statement, `SELECT`/`WITH`/`PRAGMA`/`DESCRIBE`/`SHOW`/`EXPLAIN` only
- mutating keywords are blocklisted; `--` comments are stripped before scanning
- a LIMIT is appended automatically if you forget one

## Auto line chart

When a result contains a **time-like column plus numeric columns**, a line
chart is rendered above the table (toggle with the `chart` button):

- **X axis** — a column named like `ts / time / timestamp / date`, or any
  column whose values parse as epoch nanoseconds/microseconds/milliseconds/
  seconds or ISO datetime strings.
- **Lines** — every remaining numeric column becomes a line, up to 8.
  Select multiple numeric columns to get multiple lines. Columns that look
  like epoch timestamps are skipped as y-values.
- Rows are sorted by time automatically; `ORDER BY` your time column anyway.

Tips: cast text-typed numbers with `::DOUBLE`, and alias columns
(`AS p95_ms`) to label the lines.

## Load-testing examples

The built-in examples assume aiolocust telemetry (see
[Load Testing](load-testing.md)). Two quirks matter:

- `locust.current_users` is a gauge stored in `value_int` — use
  `coalesce(value_double, value_int)`.
- `locust.client.duration` histograms are **cumulative**
  (`aggregation_temporality = 2`): bounds and bucket_counts are JSON strings
  (cast with `::DOUBLE[]`) and rates/percentiles must diff consecutive
  snapshots.

Example — concurrent users over time (gauge stored in `value_int`):

```sql
SELECT to_timestamp(ts_ns / 1e9) AS ts,
       coalesce(value_double, value_int) AS users
FROM metric_points
WHERE metric_name = 'locust.current_users'
ORDER BY ts_ns
```

The panel's built-in examples also include a **latency p95 vs time** query
(latest cumulative snapshot per 30s bucket, p95 read off the bucket counts)
and a **requests per second** query (rate = delta of `hist_count` between
consecutive exports divided by the time between them, per endpoint series).
Open the SQL panel's example dropdown to run them as-is.
