//! Flat row types written into DuckDB, plus the schema DDL and insert helpers.

use duckdb::{Connection, params};

pub const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS spans (
    trace_id                 VARCHAR NOT NULL,
    span_id                  VARCHAR NOT NULL,
    parent_span_id           VARCHAR,
    trace_state              VARCHAR,
    span_name                VARCHAR NOT NULL,
    span_kind                INTEGER NOT NULL,
    start_ns                 BIGINT NOT NULL,
    end_ns                   BIGINT NOT NULL,
    duration_ms              DOUBLE NOT NULL,
    service_name             VARCHAR NOT NULL,
    scope_name               VARCHAR,
    scope_version            VARCHAR,
    schema_url               VARCHAR,
    resource_attributes      VARCHAR,
    span_attributes          VARCHAR,
    status_code              INTEGER NOT NULL,
    status_message           VARCHAR,
    events                   VARCHAR,
    links                    VARCHAR,
    dropped_attributes_count INTEGER NOT NULL DEFAULT 0,
    dropped_events_count     INTEGER NOT NULL DEFAULT 0,
    dropped_links_count      INTEGER NOT NULL DEFAULT 0,
    flags                    BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS log_records (
    time_ns             BIGINT NOT NULL,
    observed_time_ns    BIGINT,
    severity_text       VARCHAR,
    severity_number     INTEGER,
    service_name        VARCHAR NOT NULL,
    scope_name          VARCHAR,
    scope_version       VARCHAR,
    schema_url          VARCHAR,
    body                VARCHAR,
    trace_id            VARCHAR,
    span_id             VARCHAR,
    event_name          VARCHAR,
    log_attributes      VARCHAR,
    resource_attributes VARCHAR,
    flags               BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS metric_points (
    metric_name                VARCHAR NOT NULL,
    metric_type                VARCHAR NOT NULL,
    metric_unit                VARCHAR,
    metric_description         VARCHAR,
    aggregation_temporality    INTEGER,
    is_monotonic               BOOLEAN,
    service_name               VARCHAR NOT NULL,
    scope_name                 VARCHAR,
    scope_version              VARCHAR,
    schema_url                 VARCHAR,
    resource_attributes        VARCHAR,
    series_attributes          VARCHAR,
    metric_metadata            VARCHAR,
    exemplars                  VARCHAR,
    start_ns                   BIGINT,
    ts_ns                      BIGINT NOT NULL,
    value_double               DOUBLE,
    value_int                  BIGINT,
    hist_count                 BIGINT,
    hist_sum                   DOUBLE,
    hist_min                   DOUBLE,
    hist_max                   DOUBLE,
    hist_bounds                VARCHAR,
    hist_bucket_counts         VARCHAR,
    exp_zero_count             BIGINT,
    exp_scale                  INTEGER,
    exp_zero_threshold         DOUBLE,
    exp_buckets                VARCHAR,
    summary_count              BIGINT,
    summary_sum                DOUBLE,
    summary_quantiles          VARCHAR,
    flags                      BIGINT NOT NULL DEFAULT 0
);
"#;

/// A flattened span row. Nanosecond timestamps are stored as BIGINT.
#[derive(Debug, Clone)]
pub struct SpanRow {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub trace_state: Option<String>,
    pub span_name: String,
    pub span_kind: i32,
    pub start_ns: i64,
    pub end_ns: i64,
    pub service_name: String,
    pub scope_name: Option<String>,
    pub scope_version: Option<String>,
    pub schema_url: Option<String>,
    pub resource_attributes: Option<String>,
    pub span_attributes: Option<String>,
    pub status_code: i32,
    pub status_message: Option<String>,
    pub events: Option<String>,
    pub links: Option<String>,
    pub dropped_attributes_count: i32,
    pub dropped_events_count: i32,
    pub dropped_links_count: i32,
    pub flags: i64,
}

/// A flattened log record row.
#[derive(Debug, Clone)]
pub struct LogRow {
    pub time_ns: i64,
    pub observed_time_ns: Option<i64>,
    pub severity_text: Option<String>,
    pub severity_number: i32,
    pub service_name: String,
    pub scope_name: Option<String>,
    pub scope_version: Option<String>,
    pub schema_url: Option<String>,
    pub body: Option<String>,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub event_name: Option<String>,
    pub attributes: Option<String>,
    pub resource_attributes: Option<String>,
    pub flags: i64,
}

/// A flattened metric data point row, covering every OTLP metric type.
#[derive(Debug, Clone)]
pub struct MetricPointRow {
    pub metric_name: String,
    pub metric_type: String,
    pub metric_unit: Option<String>,
    pub metric_description: Option<String>,
    pub aggregation_temporality: Option<i32>,
    pub is_monotonic: Option<bool>,
    pub service_name: String,
    pub scope_name: Option<String>,
    pub scope_version: Option<String>,
    pub schema_url: Option<String>,
    pub resource_attributes: Option<String>,
    pub series_attributes: Option<String>,
    pub metric_metadata: Option<String>,
    pub exemplars: Option<String>,
    pub start_ns: Option<i64>,
    pub ts_ns: i64,
    pub value_double: Option<f64>,
    pub value_int: Option<i64>,
    pub hist_count: Option<i64>,
    pub hist_sum: Option<f64>,
    pub hist_min: Option<f64>,
    pub hist_max: Option<f64>,
    pub hist_bounds: Option<String>,
    pub hist_bucket_counts: Option<String>,
    pub exp_zero_count: Option<i64>,
    pub exp_scale: Option<i32>,
    pub exp_zero_threshold: Option<f64>,
    pub exp_buckets: Option<String>,
    pub summary_count: Option<i64>,
    pub summary_sum: Option<f64>,
    pub summary_quantiles: Option<String>,
    pub flags: i64,
}

const SPAN_INSERT: &str = "INSERT INTO spans (trace_id, span_id, parent_span_id, trace_state, span_name, span_kind, start_ns, end_ns, duration_ms, service_name, scope_name, scope_version, schema_url, resource_attributes, span_attributes, status_code, status_message, events, links, dropped_attributes_count, dropped_events_count, dropped_links_count, flags) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";

const LOG_INSERT: &str = "INSERT INTO log_records (time_ns, observed_time_ns, severity_text, severity_number, service_name, scope_name, scope_version, schema_url, body, trace_id, span_id, event_name, log_attributes, resource_attributes, flags) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";

const METRIC_INSERT: &str = "INSERT INTO metric_points (metric_name, metric_type, metric_unit, metric_description, aggregation_temporality, is_monotonic, service_name, scope_name, scope_version, schema_url, resource_attributes, series_attributes, metric_metadata, exemplars, start_ns, ts_ns, value_double, value_int, hist_count, hist_sum, hist_min, hist_max, hist_bounds, hist_bucket_counts, exp_zero_count, exp_scale, exp_zero_threshold, exp_buckets, summary_count, summary_sum, summary_quantiles, flags) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";

pub fn insert_spans(conn: &Connection, rows: &[SpanRow]) -> duckdb::Result<()> {
    let mut stmt = conn.prepare_cached(SPAN_INSERT)?;
    for r in rows {
        let duration_ms = (r.end_ns.saturating_sub(r.start_ns)).max(0) as f64 / 1e6;
        stmt.execute(params![
            r.trace_id,
            r.span_id,
            r.parent_span_id,
            r.trace_state,
            r.span_name,
            r.span_kind,
            r.start_ns,
            r.end_ns,
            duration_ms,
            r.service_name,
            r.scope_name,
            r.scope_version,
            r.schema_url,
            r.resource_attributes,
            r.span_attributes,
            r.status_code,
            r.status_message,
            r.events,
            r.links,
            r.dropped_attributes_count,
            r.dropped_events_count,
            r.dropped_links_count,
            r.flags,
        ])?;
    }
    Ok(())
}

pub fn insert_logs(conn: &Connection, rows: &[LogRow]) -> duckdb::Result<()> {
    let mut stmt = conn.prepare_cached(LOG_INSERT)?;
    for r in rows {
        stmt.execute(params![
            r.time_ns,
            r.observed_time_ns,
            r.severity_text,
            r.severity_number,
            r.service_name,
            r.scope_name,
            r.scope_version,
            r.schema_url,
            r.body,
            r.trace_id,
            r.span_id,
            r.event_name,
            r.attributes,
            r.resource_attributes,
            r.flags,
        ])?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn insert_metrics(conn: &Connection, rows: &[MetricPointRow]) -> duckdb::Result<()> {
    let mut stmt = conn.prepare_cached(METRIC_INSERT)?;
    for r in rows {
        stmt.execute(params![
            r.metric_name,
            r.metric_type,
            r.metric_unit,
            r.metric_description,
            r.aggregation_temporality,
            r.is_monotonic,
            r.service_name,
            r.scope_name,
            r.scope_version,
            r.schema_url,
            r.resource_attributes,
            r.series_attributes,
            r.metric_metadata,
            r.exemplars,
            r.start_ns,
            r.ts_ns,
            r.value_double,
            r.value_int,
            r.hist_count,
            r.hist_sum,
            r.hist_min,
            r.hist_max,
            r.hist_bounds,
            r.hist_bucket_counts,
            r.exp_zero_count,
            r.exp_scale,
            r.exp_zero_threshold,
            r.exp_buckets,
            r.summary_count,
            r.summary_sum,
            r.summary_quantiles,
            r.flags,
        ])?;
    }
    Ok(())
}
