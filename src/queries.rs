//! Read queries and response DTOs for the REST API.

use duckdb::types::{TimeUnit, Value as PVal, ValueRef};
use duckdb::{Connection, Row, params_from_iter};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::sqlguard;

// ---------------------------------------------------------------------------
// Params
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct TracesParams {
    pub service: Option<String>,
    pub q: Option<String>,
    pub start_ns: Option<i64>,
    pub end_ns: Option<i64>,
    pub min_duration_ms: Option<f64>,
    pub errors_only: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct LogsParams {
    pub service: Option<String>,
    pub q: Option<String>,
    pub severity: Option<String>,
    pub start_ns: Option<i64>,
    pub end_ns: Option<i64>,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct MetricsParams {
    pub service: Option<String>,
    pub q: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct MetricDetailParams {
    pub service: Option<String>,
    pub start_ns: Option<i64>,
    pub end_ns: Option<i64>,
    pub limit: Option<i64>,
}

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct TracesResponse {
    pub total: i64,
    pub traces: Vec<TraceSummary>,
}

#[derive(Debug, Serialize)]
pub struct TraceSummary {
    pub trace_id: String,
    pub root_name: Option<String>,
    pub service: Option<String>,
    pub services: Vec<String>,
    pub start_ns: String,
    pub duration_ns: String,
    pub span_count: i64,
    pub error_count: i64,
}

#[derive(Debug, Serialize)]
pub struct TraceDetail {
    pub trace_id: String,
    pub start_ns: String,
    pub duration_ns: String,
    pub span_count: i64,
    pub error_count: i64,
    pub services: Vec<String>,
    pub spans: Vec<SpanDto>,
}

#[derive(Debug, Serialize)]
pub struct SpanDto {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub trace_state: Option<String>,
    pub span_name: String,
    pub span_kind: i32,
    pub start_ns: String,
    pub end_ns: String,
    pub duration_ms: f64,
    pub service_name: String,
    pub scope_name: Option<String>,
    pub scope_version: Option<String>,
    pub schema_url: Option<String>,
    pub resource_attributes: Value,
    pub attributes: Value,
    pub status_code: i32,
    pub status_message: Option<String>,
    pub events: Value,
    pub links: Value,
    pub dropped_attributes_count: i64,
    pub dropped_events_count: i64,
    pub dropped_links_count: i64,
    pub flags: i64,
}

#[derive(Debug, Serialize)]
pub struct LogsResponse {
    pub total: i64,
    pub logs: Vec<LogDto>,
}

#[derive(Debug, Serialize)]
pub struct LogDto {
    pub time_ns: String,
    pub observed_time_ns: Option<String>,
    pub severity_text: Option<String>,
    pub severity_number: i64,
    pub service_name: String,
    pub scope_name: Option<String>,
    pub scope_version: Option<String>,
    pub schema_url: Option<String>,
    pub body: Value,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub event_name: Option<String>,
    pub attributes: Value,
    pub resource_attributes: Value,
    pub flags: i64,
}

#[derive(Debug, Serialize)]
pub struct MetricsResponse {
    pub metrics: Vec<MetricSummary>,
}

#[derive(Debug, Serialize)]
pub struct MetricSummary {
    pub name: String,
    pub metric_type: String,
    pub unit: Option<String>,
    pub description: Option<String>,
    pub point_count: i64,
    pub service_count: i64,
    pub services: Vec<String>,
    pub first_ns: String,
    pub last_ns: String,
    pub last_value: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct MetricDetail {
    pub name: String,
    pub metric_type: String,
    pub unit: Option<String>,
    pub description: Option<String>,
    pub points: Vec<MetricPointDto>,
}

#[derive(Debug, Serialize)]
pub struct MetricPointDto {
    pub service_name: String,
    pub scope_name: Option<String>,
    pub series_attributes: Value,
    pub resource_attributes: Value,
    pub exemplars: Value,
    pub metric_metadata: Value,
    pub aggregation_temporality: Option<i64>,
    pub is_monotonic: Option<bool>,
    pub start_ns: Option<String>,
    pub ts_ns: String,
    pub value_double: Option<f64>,
    pub value_int: Option<i64>,
    pub hist_count: Option<i64>,
    pub hist_sum: Option<f64>,
    pub hist_min: Option<f64>,
    pub hist_max: Option<f64>,
    pub hist_bounds: Value,
    pub hist_bucket_counts: Value,
    pub exp_zero_count: Option<i64>,
    pub exp_scale: Option<i64>,
    pub exp_zero_threshold: Option<f64>,
    pub exp_buckets: Value,
    pub summary_count: Option<i64>,
    pub summary_sum: Option<f64>,
    pub summary_quantiles: Value,
}

#[derive(Debug, Serialize)]
pub struct ServicesResponse {
    pub services: Vec<ServiceDto>,
}

#[derive(Debug, Serialize)]
pub struct ServiceDto {
    pub name: String,
    pub first_ns: Option<String>,
    pub last_ns: Option<String>,
    pub span_count: i64,
    pub log_count: i64,
    pub point_count: i64,
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub spans: i64,
    pub traces: i64,
    pub logs: i64,
    pub metric_points: i64,
    pub metrics: i64,
    pub services: i64,
    pub earliest_ns: Option<String>,
    pub latest_ns: Option<String>,
    pub db_file: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SchemaResponse {
    pub tables: Vec<TableDto>,
}

#[derive(Debug, Serialize)]
pub struct TableDto {
    pub name: String,
    pub row_count: i64,
    pub columns: Vec<ColumnDto>,
}

#[derive(Debug, Serialize)]
pub struct ColumnDto {
    pub name: String,
    pub r#type: String,
}

#[derive(Debug, Serialize)]
pub struct QueryResponse {
    pub columns: Vec<ColumnDto>,
    pub rows: Vec<Map<String, Value>>,
    pub row_count: usize,
    pub truncated: bool,
    pub elapsed_ms: u128,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn jopt(v: Option<String>) -> Value {
    match v {
        None => Value::Null,
        Some(s) => serde_json::from_str(&s).unwrap_or(Value::String(s)),
    }
}

fn ns(v: i64) -> String {
    v.to_string()
}

fn clamp_limit(v: Option<i64>) -> i64 {
    v.unwrap_or(100).clamp(1, 1000)
}

fn page(p_limit: Option<i64>, p_offset: Option<i64>) -> (i64, i64) {
    let limit = p_limit.unwrap_or(100).clamp(1, 1000);
    let offset = p_offset.unwrap_or(0).max(0);
    (limit, offset)
}

fn like(q: &str) -> PVal {
    PVal::Text(format!("%{q}%"))
}

fn collect_rows<T>(
    conn: &Connection,
    sql: &str,
    args: Vec<PVal>,
    mut map: impl FnMut(&Row) -> anyhow::Result<T>,
) -> anyhow::Result<Vec<T>> {
    let mut stmt = conn.prepare(sql)?;
    let mut rows = stmt.query(params_from_iter(args))?;
    let mut out = Vec::new();
    while let Some(r) = rows.next()? {
        out.push(map(r)?);
    }
    Ok(out)
}

fn query_opt<T>(
    conn: &Connection,
    sql: &str,
    args: Vec<PVal>,
    map: impl FnOnce(&Row) -> anyhow::Result<T>,
) -> anyhow::Result<Option<T>> {
    let mut stmt = conn.prepare(sql)?;
    let mut rows = stmt.query(params_from_iter(args))?;
    match rows.next()? {
        Some(r) => Ok(Some(map(r)?)),
        None => Ok(None),
    }
}

fn query_one<T>(
    conn: &Connection,
    sql: &str,
    args: Vec<PVal>,
    map: impl FnOnce(&Row) -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    query_opt(conn, sql, args, map)?.ok_or_else(|| anyhow::anyhow!("query returned no rows"))
}

fn severity_min(s: &str) -> Option<i64> {
    match s.to_ascii_lowercase().as_str() {
        "trace" => Some(1),
        "debug" => Some(5),
        "info" => Some(9),
        "warn" | "warning" => Some(13),
        "error" => Some(17),
        "fatal" => Some(21),
        other => other.parse::<i64>().ok(),
    }
}

// ---------------------------------------------------------------------------
// Traces
// ---------------------------------------------------------------------------

const TRACE_AGG: &str = "trace_id, min(start_ns) AS start_ns, max(end_ns) - min(start_ns) AS duration_ns, \
     count(*) AS span_count, count(*) FILTER (WHERE status_code = 2) AS error_count, \
     arg_min(span_name, start_ns) AS root_name, arg_min(service_name, start_ns) AS service, \
     string_agg(DISTINCT service_name, ',') AS services";

pub fn traces_list(conn: &Connection, p: &TracesParams) -> anyhow::Result<TracesResponse> {
    let mut clauses: Vec<&str> = Vec::new();
    let mut args: Vec<PVal> = Vec::new();
    if let Some(v) = p.start_ns {
        clauses.push("start_ns >= ?");
        args.push(PVal::BigInt(v));
    }
    if let Some(v) = p.end_ns {
        clauses.push("end_ns <= ?");
        args.push(PVal::BigInt(v));
    }
    if let Some(s) = &p.service {
        clauses.push("service_name = ?");
        args.push(PVal::Text(s.clone()));
    }
    if let Some(q) = &p.q {
        clauses.push("(span_name ILIKE ? OR trace_id ILIKE ?)");
        args.push(like(q));
        args.push(like(q));
    }
    if let Some(d) = p.min_duration_ms {
        clauses.push("(end_ns - start_ns) >= ?");
        args.push(PVal::BigInt((d * 1e6) as i64));
    }
    let where_sql = if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };
    let having = if p.errors_only.unwrap_or(false) {
        "HAVING count(*) FILTER (WHERE status_code = 2) > 0"
    } else {
        ""
    };
    let (limit, offset) = page(p.limit, p.offset);

    let list_sql = format!(
        "SELECT {TRACE_AGG} FROM spans {where_sql} GROUP BY trace_id {having} \
         ORDER BY start_ns DESC LIMIT {limit} OFFSET {offset}"
    );
    let count_sql = format!(
        "SELECT count(*) FROM (SELECT trace_id FROM spans {where_sql} GROUP BY trace_id {having})"
    );

    let total: i64 = query_one(conn, &count_sql, args.clone(), |r| Ok(r.get::<_, i64>(0)?))?;
    let traces = collect_rows(conn, &list_sql, args, trace_summary_row)?;
    Ok(TracesResponse { total, traces })
}

fn trace_summary_row(r: &Row) -> anyhow::Result<TraceSummary> {
    Ok(TraceSummary {
        trace_id: r.get("trace_id")?,
        root_name: r.get("root_name")?,
        service: r.get("service")?,
        services: split_services(r.get("services")?),
        start_ns: ns(r.get::<_, i64>("start_ns")?),
        duration_ns: ns(r.get::<_, i64>("duration_ns")?),
        span_count: r.get("span_count")?,
        error_count: r.get("error_count")?,
    })
}

fn split_services(v: Option<String>) -> Vec<String> {
    v.map(|s| s.split(',').map(str::to_string).collect())
        .unwrap_or_default()
}

pub fn trace_detail(conn: &Connection, trace_id: &str) -> anyhow::Result<Option<TraceDetail>> {
    let agg_sql = format!("SELECT {TRACE_AGG} FROM spans WHERE trace_id = ? GROUP BY trace_id");
    let summary = query_opt(
        conn,
        &agg_sql,
        vec![PVal::Text(trace_id.to_string())],
        trace_summary_row,
    )?;
    let Some(summary) = summary else {
        return Ok(None);
    };

    let spans = collect_rows(
        conn,
        "SELECT * FROM spans WHERE trace_id = ? ORDER BY start_ns, span_id",
        vec![PVal::Text(trace_id.to_string())],
        span_row,
    )?;

    Ok(Some(TraceDetail {
        trace_id: summary.trace_id,
        start_ns: summary.start_ns,
        duration_ns: summary.duration_ns,
        span_count: summary.span_count,
        error_count: summary.error_count,
        services: summary.services,
        spans,
    }))
}

fn span_row(r: &Row) -> anyhow::Result<SpanDto> {
    Ok(SpanDto {
        trace_id: r.get("trace_id")?,
        span_id: r.get("span_id")?,
        parent_span_id: r.get("parent_span_id")?,
        trace_state: r.get("trace_state")?,
        span_name: r.get("span_name")?,
        span_kind: r.get("span_kind")?,
        start_ns: ns(r.get::<_, i64>("start_ns")?),
        end_ns: ns(r.get::<_, i64>("end_ns")?),
        duration_ms: r.get("duration_ms")?,
        service_name: r.get("service_name")?,
        scope_name: r.get("scope_name")?,
        scope_version: r.get("scope_version")?,
        schema_url: r.get("schema_url")?,
        resource_attributes: jopt(r.get("resource_attributes")?),
        attributes: jopt(r.get("span_attributes")?),
        status_code: r.get("status_code")?,
        status_message: r.get("status_message")?,
        events: jopt(r.get("events")?),
        links: jopt(r.get("links")?),
        dropped_attributes_count: r.get("dropped_attributes_count")?,
        dropped_events_count: r.get("dropped_events_count")?,
        dropped_links_count: r.get("dropped_links_count")?,
        flags: r.get("flags")?,
    })
}

// ---------------------------------------------------------------------------
// Logs
// ---------------------------------------------------------------------------

pub fn logs_list(conn: &Connection, p: &LogsParams) -> anyhow::Result<LogsResponse> {
    let mut clauses: Vec<&str> = Vec::new();
    let mut args: Vec<PVal> = Vec::new();
    if let Some(v) = p.start_ns {
        clauses.push("time_ns >= ?");
        args.push(PVal::BigInt(v));
    }
    if let Some(v) = p.end_ns {
        clauses.push("time_ns <= ?");
        args.push(PVal::BigInt(v));
    }
    if let Some(s) = &p.service {
        clauses.push("service_name = ?");
        args.push(PVal::Text(s.clone()));
    }
    if let Some(sev) = &p.severity
        && let Some(min) = severity_min(sev)
    {
        clauses.push("severity_number >= ?");
        args.push(PVal::BigInt(min));
    }
    if let Some(q) = &p.q {
        clauses.push("(body ILIKE ? OR log_attributes ILIKE ? OR severity_text ILIKE ? OR event_name ILIKE ?)");
        args.push(like(q));
        args.push(like(q));
        args.push(like(q));
        args.push(like(q));
    }
    if let Some(t) = &p.trace_id {
        clauses.push("trace_id = ?");
        args.push(PVal::Text(t.clone()));
    }
    if let Some(s) = &p.span_id {
        clauses.push("span_id = ?");
        args.push(PVal::Text(s.clone()));
    }
    let where_sql = if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };
    let (limit, offset) = page(p.limit, p.offset);

    let list_sql = format!(
        "SELECT * FROM log_records {where_sql} ORDER BY time_ns DESC LIMIT {limit} OFFSET {offset}"
    );
    let count_sql = format!("SELECT count(*) FROM log_records {where_sql}");

    let total: i64 = query_one(conn, &count_sql, args.clone(), |r| Ok(r.get::<_, i64>(0)?))?;
    let logs = collect_rows(conn, &list_sql, args, log_row)?;
    Ok(LogsResponse { total, logs })
}

fn log_row(r: &Row) -> anyhow::Result<LogDto> {
    Ok(LogDto {
        time_ns: ns(r.get::<_, i64>("time_ns")?),
        observed_time_ns: r.get::<_, Option<i64>>("observed_time_ns")?.map(ns),
        severity_text: r.get("severity_text")?,
        severity_number: r.get("severity_number")?,
        service_name: r.get("service_name")?,
        scope_name: r.get("scope_name")?,
        scope_version: r.get("scope_version")?,
        schema_url: r.get("schema_url")?,
        body: jopt(r.get("body")?),
        trace_id: r.get("trace_id")?,
        span_id: r.get("span_id")?,
        event_name: r.get("event_name")?,
        attributes: jopt(r.get("log_attributes")?),
        resource_attributes: jopt(r.get("resource_attributes")?),
        flags: r.get("flags")?,
    })
}

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

pub fn metrics_list(conn: &Connection, p: &MetricsParams) -> anyhow::Result<MetricsResponse> {
    let mut clauses: Vec<&str> = Vec::new();
    let mut args: Vec<PVal> = Vec::new();
    if let Some(s) = &p.service {
        clauses.push("service_name = ?");
        args.push(PVal::Text(s.clone()));
    }
    if let Some(q) = &p.q {
        clauses.push("(metric_name ILIKE ? OR metric_description ILIKE ?)");
        args.push(like(q));
        args.push(like(q));
    }
    let where_sql = if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    };
    let limit = clamp_limit(p.limit);
    let sql = format!(
        "SELECT metric_name, metric_type, metric_unit, metric_description, \
         count(*) AS point_count, count(DISTINCT service_name) AS service_count, \
         string_agg(DISTINCT service_name, ',') AS services, \
         min(ts_ns) AS first_ns, max(ts_ns) AS last_ns, \
         arg_max(value_double, ts_ns) AS last_value \
         FROM metric_points {where_sql} \
         GROUP BY metric_name, metric_type, metric_unit, metric_description \
         ORDER BY metric_name LIMIT {limit}"
    );
    let metrics = collect_rows(conn, &sql, args, |r| {
        Ok(MetricSummary {
            name: r.get("metric_name")?,
            metric_type: r.get("metric_type")?,
            unit: r.get("metric_unit")?,
            description: r.get("metric_description")?,
            point_count: r.get("point_count")?,
            service_count: r.get("service_count")?,
            services: split_services(r.get("services")?),
            first_ns: ns(r.get::<_, i64>("first_ns")?),
            last_ns: ns(r.get::<_, i64>("last_ns")?),
            last_value: r.get("last_value")?,
        })
    })?;
    Ok(MetricsResponse { metrics })
}

pub fn metric_detail(
    conn: &Connection,
    name: &str,
    p: &MetricDetailParams,
) -> anyhow::Result<Option<MetricDetail>> {
    let mut clauses: Vec<&str> = vec!["metric_name = ?"];
    let mut args: Vec<PVal> = vec![PVal::Text(name.to_string())];
    if let Some(s) = &p.service {
        clauses.push("service_name = ?");
        args.push(PVal::Text(s.clone()));
    }
    if let Some(v) = p.start_ns {
        clauses.push("ts_ns >= ?");
        args.push(PVal::BigInt(v));
    }
    if let Some(v) = p.end_ns {
        clauses.push("ts_ns <= ?");
        args.push(PVal::BigInt(v));
    }
    let where_sql = format!("WHERE {}", clauses.join(" AND "));
    let limit = p.limit.unwrap_or(5000).clamp(1, 20000);

    let meta = query_opt(
        conn,
        &format!(
            "SELECT metric_name, metric_type, metric_unit, metric_description \
             FROM metric_points {where_sql} LIMIT 1"
        ),
        args.clone(),
        |r| {
            Ok::<_, anyhow::Error>((
                r.get::<_, String>("metric_type")?,
                r.get::<_, Option<String>>("metric_unit")?,
                r.get::<_, Option<String>>("metric_description")?,
            ))
        },
    )?;
    let Some((metric_type, unit, description)) = meta else {
        return Ok(None);
    };

    let points = collect_rows(
        conn,
        &format!(
            "SELECT service_name, scope_name, series_attributes, resource_attributes, exemplars, \
             metric_metadata, aggregation_temporality, is_monotonic, start_ns, ts_ns, \
             value_double, value_int, hist_count, hist_sum, hist_min, hist_max, hist_bounds, \
             hist_bucket_counts, exp_zero_count, exp_scale, exp_zero_threshold, exp_buckets, \
             summary_count, summary_sum, summary_quantiles \
             FROM metric_points {where_sql} ORDER BY ts_ns LIMIT {limit}"
        ),
        args,
        metric_point_row,
    )?;

    Ok(Some(MetricDetail {
        name: name.to_string(),
        metric_type,
        unit,
        description,
        points,
    }))
}

fn metric_point_row(r: &Row) -> anyhow::Result<MetricPointDto> {
    Ok(MetricPointDto {
        service_name: r.get("service_name")?,
        scope_name: r.get("scope_name")?,
        series_attributes: jopt(r.get("series_attributes")?),
        resource_attributes: jopt(r.get("resource_attributes")?),
        exemplars: jopt(r.get("exemplars")?),
        metric_metadata: jopt(r.get("metric_metadata")?),
        aggregation_temporality: r.get("aggregation_temporality")?,
        is_monotonic: r.get("is_monotonic")?,
        start_ns: r.get::<_, Option<i64>>("start_ns")?.map(ns),
        ts_ns: ns(r.get::<_, i64>("ts_ns")?),
        value_double: r.get("value_double")?,
        value_int: r.get("value_int")?,
        hist_count: r.get("hist_count")?,
        hist_sum: r.get("hist_sum")?,
        hist_min: r.get("hist_min")?,
        hist_max: r.get("hist_max")?,
        hist_bounds: jopt(r.get("hist_bounds")?),
        hist_bucket_counts: jopt(r.get("hist_bucket_counts")?),
        exp_zero_count: r.get("exp_zero_count")?,
        exp_scale: r.get("exp_scale")?,
        exp_zero_threshold: r.get("exp_zero_threshold")?,
        exp_buckets: jopt(r.get("exp_buckets")?),
        summary_count: r.get("summary_count")?,
        summary_sum: r.get("summary_sum")?,
        summary_quantiles: jopt(r.get("summary_quantiles")?),
    })
}

// ---------------------------------------------------------------------------
// Services / stats / schema
// ---------------------------------------------------------------------------

pub fn services(conn: &Connection) -> anyhow::Result<ServicesResponse> {
    let services = collect_rows(
        conn,
        "SELECT service_name, min(first_ns) AS first_ns, max(last_ns) AS last_ns, \
         sum(spans) AS span_count, sum(logs) AS log_count, sum(points) AS point_count \
         FROM ( \
           SELECT service_name, min(start_ns) AS first_ns, max(end_ns) AS last_ns, count(*) AS spans, 0 AS logs, 0 AS points FROM spans GROUP BY service_name \
           UNION ALL \
           SELECT service_name, min(time_ns), max(time_ns), 0, count(*), 0 FROM log_records GROUP BY service_name \
           UNION ALL \
           SELECT service_name, min(ts_ns), max(ts_ns), 0, 0, count(*) FROM metric_points GROUP BY service_name \
         ) GROUP BY service_name ORDER BY service_name",
        vec![],
        |r| {
            Ok(ServiceDto {
                name: r.get("service_name")?,
                first_ns: r.get::<_, Option<i64>>("first_ns")?.map(ns),
                last_ns: r.get::<_, Option<i64>>("last_ns")?.map(ns),
                span_count: r.get("span_count")?,
                log_count: r.get("log_count")?,
                point_count: r.get("point_count")?,
            })
        },
    )?;
    Ok(ServicesResponse { services })
}

pub fn stats(conn: &Connection, db_file: Option<&str>) -> anyhow::Result<StatsResponse> {
    query_one(
        conn,
        "SELECT \
          (SELECT count(*) FROM spans) AS spans, \
          (SELECT count(DISTINCT trace_id) FROM spans) AS traces, \
          (SELECT count(*) FROM log_records) AS logs, \
          (SELECT count(*) FROM metric_points) AS metric_points, \
          (SELECT count(DISTINCT metric_name) FROM metric_points) AS metrics, \
          (SELECT count(*) FROM (SELECT service_name FROM spans UNION SELECT service_name FROM log_records UNION SELECT service_name FROM metric_points)) AS services, \
          (SELECT min(t) FROM (SELECT min(start_ns) AS t FROM spans UNION ALL SELECT min(time_ns) FROM log_records UNION ALL SELECT min(ts_ns) FROM metric_points)) AS earliest_ns, \
          (SELECT max(t) FROM (SELECT max(end_ns) AS t FROM spans UNION ALL SELECT max(time_ns) FROM log_records UNION ALL SELECT max(ts_ns) FROM metric_points)) AS latest_ns",
        vec![],
        |r| {
            Ok(StatsResponse {
                spans: r.get("spans")?,
                traces: r.get("traces")?,
                logs: r.get("logs")?,
                metric_points: r.get("metric_points")?,
                metrics: r.get("metrics")?,
                services: r.get("services")?,
                earliest_ns: r.get::<_, Option<i64>>("earliest_ns")?.map(ns),
                latest_ns: r.get::<_, Option<i64>>("latest_ns")?.map(ns),
                db_file: db_file.map(str::to_string),
            })
        },
    )
}

// ---------------------------------------------------------------------------
// DuckDB storage stats (/api/dbstats)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct DbStatsResponse {
    pub db_file: Option<String>,
    pub file_size_bytes: Option<u64>,
    pub database_size: Option<String>,
    pub block_size: Option<i64>,
    pub total_blocks: Option<i64>,
    pub used_blocks: Option<i64>,
    pub free_blocks: Option<i64>,
    pub checkpoint_count: Option<i64>,
    pub memory_bytes: Option<i64>,
    pub tables: Vec<TableStats>,
}

#[derive(Debug, Serialize)]
pub struct TableStats {
    pub table_name: String,
    pub estimated_size: Option<i64>,
    pub column_count: Option<i64>,
    pub index_count: Option<i64>,
}

/// Storage-level stats: file size, DuckDB block usage, per-table row
/// estimates and memory. Column names from pragma_database_size() vary a bit
/// across DuckDB versions, so they are read defensively (missing → null).
pub fn db_stats(
    conn: &Connection,
    db_file: Option<&str>,
    file_size_bytes: Option<u64>,
) -> anyhow::Result<DbStatsResponse> {
    let opt = |r: &Row, name: &str| r.get::<_, Option<i64>>(name).ok().flatten();

    let (database_size, block_size, total_blocks, used_blocks, free_blocks, checkpoint_count) =
        query_opt(conn, "SELECT * FROM pragma_database_size()", vec![], |r| {
            Ok((
                r.get::<_, Option<String>>("database_size").ok().flatten(),
                opt(r, "block_size"),
                opt(r, "total_blocks"),
                opt(r, "used_blocks"),
                opt(r, "free_blocks"),
                opt(r, "checkpoint_count"),
            ))
        })?
        .unwrap_or((None, None, None, None, None, None));

    let memory_bytes = query_opt(
        conn,
        "SELECT sum(memory_usage_bytes) AS m FROM duckdb_memory()",
        vec![],
        |r| Ok(opt(r, "m")),
    )?
    .flatten();

    let tables = collect_rows(
        conn,
        "SELECT table_name, estimated_size, column_count, index_count \
         FROM duckdb_tables() WHERE schema_name = 'main' ORDER BY table_name",
        vec![],
        |r| {
            Ok(TableStats {
                table_name: r.get("table_name")?,
                estimated_size: r.get("estimated_size")?,
                column_count: r.get("column_count")?,
                index_count: r.get("index_count")?,
            })
        },
    )?;

    Ok(DbStatsResponse {
        db_file: db_file.map(str::to_string),
        file_size_bytes,
        database_size,
        block_size,
        total_blocks,
        used_blocks,
        free_blocks,
        checkpoint_count,
        memory_bytes,
        tables,
    })
}

pub fn schema(conn: &Connection) -> anyhow::Result<SchemaResponse> {
    let mut cols = collect_rows(
        conn,
        "SELECT table_name, column_name, data_type FROM information_schema.columns \
         WHERE table_schema = 'main' ORDER BY table_name, ordinal_position",
        vec![],
        |r| {
            Ok((
                r.get::<_, String>("table_name")?,
                r.get::<_, String>("column_name")?,
                r.get::<_, String>("data_type")?,
            ))
        },
    )?;

    let counts = collect_rows(
        conn,
        "SELECT 'log_records' AS t, count(*) AS c FROM log_records \
         UNION ALL SELECT 'metric_points', count(*) FROM metric_points \
         UNION ALL SELECT 'spans', count(*) FROM spans",
        vec![],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
    )?;
    let mut count_map = std::collections::HashMap::new();
    for (t, c) in counts {
        count_map.insert(t, c);
    }

    let mut tables: Vec<TableDto> = Vec::new();
    for (table, column, ty) in cols.drain(..) {
        if tables.last().is_none_or(|t| t.name != table) {
            tables.push(TableDto {
                name: table.clone(),
                row_count: *count_map.get(&table).unwrap_or(&0),
                columns: Vec::new(),
            });
        }
        tables.last_mut().unwrap().columns.push(ColumnDto {
            name: column,
            r#type: ty,
        });
    }
    Ok(SchemaResponse { tables })
}

// ---------------------------------------------------------------------------
// Arbitrary (guarded) SQL from the console
// ---------------------------------------------------------------------------

pub fn run_query(conn: &Connection, sql: &str, max_rows: usize) -> anyhow::Result<QueryResponse> {
    let sql = sqlguard::validate_readonly(sql)?;
    // Cap unbounded queries at max_rows + 1 so we can detect truncation by
    // observing the extra row, while still returning only max_rows rows.
    let sql = sqlguard::maybe_add_limit(&sql, max_rows.saturating_add(1));

    let started = std::time::Instant::now();
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query([])?;
    // Metadata must be read through the rows handle (it borrows the statement).
    let st = rows.as_ref().expect("statement handle");
    let names = st.column_names();
    let count = st.column_count();
    let columns: Vec<ColumnDto> = (0..count)
        .map(|i| ColumnDto {
            name: names.get(i).cloned().unwrap_or_else(|| format!("col{i}")),
            r#type: format!("{:?}", st.column_type(i)),
        })
        .collect();

    let mut out: Vec<Map<String, Value>> = Vec::new();
    let mut truncated = false;
    while let Some(r) = rows.next()? {
        if out.len() >= max_rows {
            truncated = true;
            break;
        }
        let mut obj = Map::new();
        for (i, c) in columns.iter().enumerate() {
            let v = r.get_ref(i)?;
            obj.insert(c.name.clone(), value_ref_to_json(v));
        }
        out.push(obj);
    }
    Ok(QueryResponse {
        columns,
        row_count: out.len(),
        rows: out,
        truncated,
        elapsed_ms: started.elapsed().as_millis(),
    })
}

fn value_ref_to_json(vr: ValueRef) -> Value {
    use duckdb::types::ValueRef as V;
    match vr {
        V::Null => Value::Null,
        V::Boolean(b) => json!(b),
        V::TinyInt(i) => json!(i),
        V::SmallInt(i) => json!(i),
        V::Int(i) => json!(i),
        V::BigInt(i) => json!(i),
        V::UTinyInt(i) => json!(i),
        V::USmallInt(i) => json!(i),
        V::UInt(i) => json!(i),
        V::UBigInt(i) => json!(i),
        V::HugeInt(i) => Value::String(i.to_string()),
        V::UHugeInt(i) => Value::String(i.to_string()),
        V::Float(f) => dnum(f as f64),
        V::Double(f) => dnum(f),
        V::Text(t) => json!(String::from_utf8_lossy(t)),
        V::Blob(b) => json!(hex::encode(b)),
        V::Timestamp(unit, v) => json!(timestamp_to_string(unit, v)),
        V::Date32(days) => json!(date_to_string(days)),
        V::Time64(unit, v) => json!(format!("{unit:?}:{v}")),
        V::Interval {
            months,
            days,
            nanos,
        } => json!(format!("{months}mo {days}d {nanos}ns")),
        other => json!(format!("{other:?}")),
    }
}

fn dnum(v: f64) -> Value {
    if v.is_finite() {
        json!(v)
    } else {
        json!(v.to_string())
    }
}

fn timestamp_to_string(unit: TimeUnit, v: i64) -> String {
    let (secs, nanos) = match unit {
        TimeUnit::Second => (v, 0u32),
        TimeUnit::Millisecond => (
            v.div_euclid(1_000),
            (v.rem_euclid(1_000) * 1_000_000) as u32,
        ),
        TimeUnit::Microsecond => (
            v.div_euclid(1_000_000),
            (v.rem_euclid(1_000_000) * 1_000) as u32,
        ),
        TimeUnit::Nanosecond => (
            v.div_euclid(1_000_000_000),
            v.rem_euclid(1_000_000_000) as u32,
        ),
    };
    chrono::DateTime::from_timestamp(secs, nanos)
        .map(|d| d.format("%Y-%m-%dT%H:%M:%S%.9fZ").to_string())
        .unwrap_or_else(|| v.to_string())
}

fn date_to_string(days: i32) -> String {
    chrono::DateTime::from_timestamp((days as i64) * 86_400, 0)
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| days.to_string())
}
