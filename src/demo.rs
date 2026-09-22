//! Demo telemetry seeding, so the UI can be explored without wiring up a
//! real application. Uses the exact same storage path as OTLP ingestion.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use rand::rngs::ThreadRng;
use rand::{Rng, RngExt};
use serde_json::json;

use crate::db::Db;
use crate::rows::{LogRow, MetricPointRow, SpanRow};

fn now_ns() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}

fn attrs(pairs: &[(&str, serde_json::Value)]) -> Option<String> {
    let mut m = serde_json::Map::new();
    for (k, v) in pairs {
        m.insert(k.to_string(), v.clone());
    }
    Some(serde_json::Value::Object(m).to_string())
}

fn hex32(rng: &mut impl Rng) -> String {
    let bytes: Vec<u8> = (0..16).map(|_| rng.random::<u8>()).collect();
    hex::encode(bytes)
}

fn hex16(rng: &mut impl Rng) -> String {
    let bytes: Vec<u8> = (0..8).map(|_| rng.random::<u8>()).collect();
    hex::encode(bytes)
}

const ROUTES: &[&str] = &[
    "GET /api/users",
    "GET /api/orders",
    "POST /api/orders",
    "GET /api/products",
    "POST /api/checkout",
];

const DB_QUERIES: &[&str] = &[
    "SELECT * FROM users WHERE id = ?",
    "SELECT * FROM orders WHERE user_id = ?",
    "INSERT INTO orders ...",
    "SELECT SUM(total) FROM orders GROUP BY user_id",
];

/// Seed demo data if the database is empty. Returns the number of rows
/// inserted (0 when skipped).
pub async fn seed_if_empty(db: &Arc<Db>) -> anyhow::Result<usize> {
    let existing: i64 = db
        .read(|conn| {
            let s: i64 = conn.query_row(
                "SELECT (SELECT count(*) FROM spans) + (SELECT count(*) FROM log_records) + (SELECT count(*) FROM metric_points)",
                [],
                |r| r.get(0),
            )?;
            Ok(s)
        })
        .await?;
    if existing > 0 {
        return Ok(0);
    }
    seed(db).await
}

#[allow(clippy::too_many_arguments)]
fn make_span(
    rng: &mut impl Rng,
    trace_id: &str,
    name: String,
    kind: i32,
    service: &str,
    parent: Option<String>,
    ms: f64,
    offset_ns: i64,
    error: bool,
) -> SpanRow {
    SpanRow {
        trace_id: trace_id.to_string(),
        span_id: hex16(rng),
        parent_span_id: parent,
        trace_state: None,
        span_name: name,
        span_kind: kind,
        start_ns: offset_ns,
        end_ns: offset_ns + (ms * 1e6) as i64,
        service_name: service.to_string(),
        scope_name: Some("oteldemo.web".to_string()),
        scope_version: Some("1.0.0".to_string()),
        schema_url: Some("https://opentelemetry.io/schemas/1.16.0".to_string()),
        resource_attributes: attrs(&[
            ("service.name", json!(service)),
            ("service.version", json!("1.4.2")),
            ("deployment.environment", json!("production")),
        ]),
        span_attributes: None,
        status_code: if error { 2 } else { 1 },
        status_message: error.then(|| "upstream connection refused".to_string()),
        events: None,
        links: None,
        dropped_attributes_count: 0,
        dropped_events_count: 0,
        dropped_links_count: 0,
        flags: 1,
    }
}

/// Generate all demo rows up front (pure, synchronous) so no `!Send` state
/// (ThreadRng) is ever held across an await.
fn build_demo_data() -> (Vec<SpanRow>, Vec<LogRow>, Vec<MetricPointRow>) {
    let mut rng = rand::rng();
    let mut spans: Vec<SpanRow> = Vec::new();
    let mut logs: Vec<LogRow> = Vec::new();
    let mut points: Vec<MetricPointRow> = Vec::new();
    let base = now_ns();

    // -----------------------------------------------------------------
    // Traces: gateway -> billing/postgres/redis trees
    // -----------------------------------------------------------------
    for i in 0..40 {
        let trace_id = hex32(&mut rng);
        let start = base - rng.random_range(0..30 * 60) * 1_000_000_000;
        let route = ROUTES[rng.random_range(0..ROUTES.len())];
        let is_error = rng.random_bool(0.18);
        let gateway_ms = rng.random_range(15.0..280.0);
        let mut cursor = start;
        let method = route.split(' ').next().unwrap_or("GET");
        let path = route.split(' ').nth(1).unwrap_or("/");

        let mut s = make_span(
            &mut rng,
            &trace_id,
            format!("{method} {path}"),
            2,
            "api-gateway",
            None,
            gateway_ms,
            cursor,
            is_error,
        );
        s.span_attributes = attrs(&[
            ("http.request.method", json!(method)),
            ("url.path", json!(path)),
            (
                "http.response.status_code",
                json!(if is_error { 500 } else { 200 }),
            ),
            ("user.id", json!(rng.random_range(1..1000))),
        ]);
        let root_id = s.span_id.clone();
        spans.push(s);

        // auth child
        let auth_ms = rng.random_range(0.8..6.0);
        let mut a = make_span(
            &mut rng,
            &trace_id,
            "authorize".to_string(),
            3,
            "auth-service",
            Some(root_id.clone()),
            auth_ms,
            cursor,
            false,
        );
        a.span_attributes = attrs(&[
            ("grpc.method", json!("auth.Authorize")),
            ("auth.provider", json!("oidc")),
        ]);
        spans.push(a);
        cursor += (auth_ms * 1e6) as i64;

        // downstream billing server span sometimes
        if rng.random_bool(0.6) {
            let bill_ms = rng.random_range(5.0..120.0);
            let mut b = make_span(
                &mut rng,
                &trace_id,
                "POST /billing/charge".to_string(),
                2,
                "billing-service",
                Some(root_id.clone()),
                bill_ms,
                cursor,
                false,
            );
            b.span_attributes = attrs(&[
                ("messaging.destination", json!("billing.queue")),
                ("amount", json!(rng.random_range(1..250))),
            ]);
            let bill_id = b.span_id.clone();
            spans.push(b);

            let q = DB_QUERIES[rng.random_range(0..DB_QUERIES.len())].to_string();
            let qms = rng.random_range(1.0..25.0);
            let mut qs = make_span(
                &mut rng,
                &trace_id,
                q.clone(),
                3,
                "postgres",
                Some(bill_id),
                qms,
                cursor,
                false,
            );
            qs.span_attributes = attrs(&[
                ("db.system", json!("postgresql")),
                ("db.statement", json!(q)),
            ]);
            spans.push(qs);
            cursor += (bill_ms * 1e6) as i64;
        }

        // db child on gateway
        let q = DB_QUERIES[rng.random_range(0..DB_QUERIES.len())].to_string();
        let qms = rng.random_range(1.0..30.0);
        let mut qs = make_span(
            &mut rng,
            &trace_id,
            q.clone(),
            3,
            "postgres",
            Some(root_id.clone()),
            qms,
            cursor,
            false,
        );
        qs.span_attributes = attrs(&[
            ("db.system", json!("postgresql")),
            ("db.statement", json!(q)),
        ]);
        spans.push(qs);

        // redis child sometimes
        if rng.random_bool(0.5) {
            let rms = rng.random_range(0.2..3.0);
            let mut rs = make_span(
                &mut rng,
                &trace_id,
                "GET cache:user".to_string(),
                3,
                "redis",
                Some(root_id.clone()),
                rms,
                cursor,
                false,
            );
            rs.span_attributes = attrs(&[
                ("db.system", json!("redis")),
                ("cache.hit", json!(rng.random_bool(0.8))),
            ]);
            spans.push(rs);
        }

        // logs for this trace
        let log_jitter =
            |rng: &mut ThreadRng| rng.random_range(0..(gateway_ms as i64).max(1)) * 1_000_000;
        let make_trace_log = |sev_text: &str,
                              sev_num: i32,
                              body: &str,
                              extra: Vec<(&str, serde_json::Value)>,
                              rng: &mut ThreadRng| {
            let mut pairs = vec![
                ("request_id", json!(trace_id[..16.min(trace_id.len())])),
                ("http.route", json!(route)),
            ];
            pairs.extend(extra);
            LogRow {
                time_ns: start + log_jitter(rng),
                observed_time_ns: None,
                severity_text: Some(sev_text.to_string()),
                severity_number: sev_num,
                service_name: "api-gateway".to_string(),
                scope_name: Some("oteldemo.web".to_string()),
                scope_version: Some("1.0.0".to_string()),
                schema_url: None,
                body: Some(json!(body).to_string()),
                trace_id: Some(trace_id.clone()),
                span_id: Some(root_id.clone()),
                event_name: None,
                attributes: attrs(&pairs),
                resource_attributes: attrs(&[
                    ("service.name", json!("api-gateway")),
                    ("deployment.environment", json!("production")),
                ]),
                flags: 0,
            }
        };

        logs.push(make_trace_log(
            "INFO",
            9,
            "request started",
            vec![],
            &mut rng,
        ));
        if is_error {
            logs.push(make_trace_log(
                "ERROR",
                17,
                "upstream billing call failed: connection refused",
                vec![],
                &mut rng,
            ));
            logs.push(make_trace_log(
                "WARN",
                13,
                "retrying with backoff",
                vec![("retry.count", json!(2))],
                &mut rng,
            ));
        } else if gateway_ms > 200.0 {
            logs.push(make_trace_log(
                "WARN",
                13,
                "slow request detected",
                vec![("duration_ms", json!(gateway_ms.round()))],
                &mut rng,
            ));
        } else {
            logs.push(make_trace_log(
                "DEBUG",
                5,
                "request completed",
                vec![("duration_ms", json!(gateway_ms.round()))],
                &mut rng,
            ));
        }
        if i % 7 == 0 {
            logs.push(make_trace_log(
                "INFO",
                9,
                "user session refreshed",
                vec![("user.id", json!(rng.random_range(1..1000)))],
                &mut rng,
            ));
        }
    }

    // standalone background logs
    for i in 0..30 {
        let sev = match i % 5 {
            0 => ("WARN", 13, "certificate expires in 12 days"),
            1 => ("ERROR", 17, "failed to flush metrics batch"),
            2 => ("INFO", 9, "background job finished"),
            _ => ("DEBUG", 5, "cache eviction sweep"),
        };
        logs.push(LogRow {
            time_ns: base - rng.random_range(0..30 * 60) * 1_000_000_000,
            observed_time_ns: None,
            severity_text: Some(sev.0.to_string()),
            severity_number: sev.1,
            service_name: if i % 2 == 0 {
                "api-gateway".to_string()
            } else {
                "billing-service".to_string()
            },
            scope_name: Some("oteldemo.jobs".to_string()),
            scope_version: Some("1.0.0".to_string()),
            schema_url: None,
            body: Some(json!(sev.2).to_string()),
            trace_id: None,
            span_id: None,
            event_name: None,
            attributes: attrs(&[("job.name", json!("cleanup")), ("attempt", json!(i % 3))]),
            resource_attributes: attrs(&[
                ("service.name", json!("api-gateway")),
                ("deployment.environment", json!("production")),
            ]),
            flags: 0,
        });
    }

    // -----------------------------------------------------------------
    // Metrics: 30 minutes of 30s-interval points
    // -----------------------------------------------------------------
    let steps = 60i64;
    let metric_res = attrs(&[
        ("service.name", json!("api-gateway")),
        ("deployment.environment", json!("production")),
    ]);

    for s in 0..steps {
        let ts = base - (steps - 1 - s) * 30 * 1_000_000_000;

        // gauge: memory usage per host
        for host in ["web-1", "web-2"] {
            points.push(MetricPointRow {
                metric_name: "system.memory.usage".to_string(),
                metric_type: "gauge".to_string(),
                metric_unit: Some("By".to_string()),
                metric_description: Some("Resident memory usage".to_string()),
                aggregation_temporality: None,
                is_monotonic: None,
                service_name: "api-gateway".to_string(),
                scope_name: Some("oteldemo.runtime".to_string()),
                scope_version: Some("1.0.0".to_string()),
                schema_url: None,
                resource_attributes: metric_res.clone(),
                series_attributes: attrs(&[("host.name", json!(host))]),
                metric_metadata: None,
                exemplars: None,
                start_ns: None,
                ts_ns: ts,
                value_double: Some(rng.random_range(700.0..900.0) * 1_000_000.0),
                value_int: None,
                hist_count: None,
                hist_sum: None,
                hist_min: None,
                hist_max: None,
                hist_bounds: None,
                hist_bucket_counts: None,
                exp_zero_count: None,
                exp_scale: None,
                exp_zero_threshold: None,
                exp_buckets: None,
                summary_count: None,
                summary_sum: None,
                summary_quantiles: None,
                flags: 0,
            });
        }

        // counter: http requests total (cumulative)
        let mut total = 100.0 + s as f64 * rng.random_range(2.0..6.0);
        for (route, code) in [
            ("GET /api/users", 200i64),
            ("GET /api/orders", 200),
            ("POST /api/checkout", 500),
        ] {
            total += rng.random_range(1.0..8.0);
            points.push(MetricPointRow {
                metric_name: "http.server.request.count".to_string(),
                metric_type: "sum".to_string(),
                metric_unit: Some("{requests}".to_string()),
                metric_description: Some("Total HTTP requests".to_string()),
                aggregation_temporality: Some(2),
                is_monotonic: Some(true),
                service_name: "api-gateway".to_string(),
                scope_name: Some("oteldemo.web".to_string()),
                scope_version: Some("1.0.0".to_string()),
                schema_url: None,
                resource_attributes: metric_res.clone(),
                series_attributes: attrs(&[
                    ("http.route", json!(route)),
                    ("http.response.status_code", json!(code)),
                ]),
                metric_metadata: None,
                exemplars: None,
                start_ns: Some(ts - steps * 30 * 1_000_000_000),
                ts_ns: ts,
                value_double: Some(total),
                value_int: None,
                hist_count: None,
                hist_sum: None,
                hist_min: None,
                hist_max: None,
                hist_bounds: None,
                hist_bucket_counts: None,
                exp_zero_count: None,
                exp_scale: None,
                exp_zero_threshold: None,
                exp_buckets: None,
                summary_count: None,
                summary_sum: None,
                summary_quantiles: None,
                flags: 0,
            });
        }

        // histogram: request durations
        let bounds = [1.0f64, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0];
        let mut bucket_counts: Vec<u64> = Vec::with_capacity(bounds.len() + 1);
        let mut remaining = 120u64;
        for _ in 0..=bounds.len() {
            let c = rng.random_range(0..(remaining / 3).max(2));
            bucket_counts.push(c);
            remaining = remaining.saturating_sub(c);
        }
        bucket_counts[bounds.len() / 2] += remaining; // bulk lands mid buckets
        let sum: f64 = bounds
            .iter()
            .zip(bucket_counts.iter())
            .map(|(b, c)| b * (*c as f64))
            .sum::<f64>()
            + bucket_counts.last().copied().unwrap_or(0) as f64 * 1200.0;
        points.push(MetricPointRow {
            metric_name: "http.server.duration".to_string(),
            metric_type: "histogram".to_string(),
            metric_unit: Some("ms".to_string()),
            metric_description: Some("HTTP request duration".to_string()),
            aggregation_temporality: Some(1),
            is_monotonic: None,
            service_name: "api-gateway".to_string(),
            scope_name: Some("oteldemo.web".to_string()),
            scope_version: Some("1.0.0".to_string()),
            schema_url: None,
            resource_attributes: metric_res.clone(),
            series_attributes: attrs(&[("http.route", json!("GET /api/users"))]),
            metric_metadata: None,
            exemplars: None,
            start_ns: Some(ts),
            ts_ns: ts,
            value_double: None,
            value_int: None,
            hist_count: Some(bucket_counts.iter().sum::<u64>() as i64),
            hist_sum: Some(sum),
            hist_min: Some(1.2),
            hist_max: Some(880.0),
            hist_bounds: Some(json!(bounds).to_string()),
            hist_bucket_counts: Some(json!(bucket_counts).to_string()),
            exp_zero_count: None,
            exp_scale: None,
            exp_zero_threshold: None,
            exp_buckets: None,
            summary_count: None,
            summary_sum: None,
            summary_quantiles: None,
            flags: 0,
        });

        // summary: cache hit ratio
        let q50 = rng.random_range(0.55..0.75);
        points.push(MetricPointRow {
            metric_name: "cache.hit_ratio".to_string(),
            metric_type: "summary".to_string(),
            metric_unit: Some("1".to_string()),
            metric_description: Some("Cache hit ratio quantiles".to_string()),
            aggregation_temporality: None,
            is_monotonic: None,
            service_name: "api-gateway".to_string(),
            scope_name: Some("oteldemo.runtime".to_string()),
            scope_version: Some("1.0.0".to_string()),
            schema_url: None,
            resource_attributes: metric_res.clone(),
            series_attributes: attrs(&[("cache.name", json!("user-cache"))]),
            metric_metadata: None,
            exemplars: None,
            start_ns: Some(ts),
            ts_ns: ts,
            value_double: None,
            value_int: None,
            hist_count: None,
            hist_sum: None,
            hist_min: None,
            hist_max: None,
            hist_bounds: None,
            hist_bucket_counts: None,
            exp_zero_count: None,
            exp_scale: None,
            exp_zero_threshold: None,
            exp_buckets: None,
            summary_count: Some(1000 + s * 37),
            summary_sum: Some(q50 * 1000.0),
            summary_quantiles: Some(
                json!([
                    {"quantile": 0.5, "value": q50},
                    {"quantile": 0.9, "value": q50 + 0.1},
                    {"quantile": 0.99, "value": q50 + 0.15},
                ])
                .to_string(),
            ),
            flags: 0,
        });

        // exponential histogram: db query duration (us)
        let scale = -4i32;
        points.push(MetricPointRow {
            metric_name: "db.query.duration".to_string(),
            metric_type: "exponential_histogram".to_string(),
            metric_unit: Some("us".to_string()),
            metric_description: Some("Database query duration".to_string()),
            aggregation_temporality: Some(2),
            is_monotonic: None,
            service_name: "postgres".to_string(),
            scope_name: Some("oteldemo.db".to_string()),
            scope_version: Some("1.0.0".to_string()),
            schema_url: None,
            resource_attributes: attrs(&[
                ("service.name", json!("postgres")),
                ("db.system", json!("postgresql")),
            ]),
            series_attributes: attrs(&[("db.operation", json!("SELECT"))]),
            metric_metadata: None,
            exemplars: None,
            start_ns: Some(ts),
            ts_ns: ts,
            value_double: None,
            value_int: None,
            hist_count: Some(90),
            hist_sum: Some(rng.random_range(400.0..900.0)),
            hist_min: Some(20.0),
            hist_max: Some(90.0),
            hist_bounds: None,
            hist_bucket_counts: None,
            exp_zero_count: Some(3),
            exp_scale: Some(scale),
            exp_zero_threshold: Some(1.0),
            exp_buckets: Some(
                json!({
                    "positive": {"offset": 6, "bucket_counts": [2, 8, 20, 30, 18, 8, 1]},
                    "negative": null
                })
                .to_string(),
            ),
            summary_count: None,
            summary_sum: None,
            summary_quantiles: None,
            flags: 0,
        });
    }

    (spans, logs, points)
}

pub async fn seed(db: &Arc<Db>) -> anyhow::Result<usize> {
    let (spans, logs, points) = build_demo_data();
    let count = spans.len() + logs.len() + points.len();
    db.insert_spans(spans).await.map_err(anyhow::Error::msg)?;
    db.insert_logs(logs).await.map_err(anyhow::Error::msg)?;
    db.insert_metrics(points)
        .await
        .map_err(anyhow::Error::msg)?;
    Ok(count)
}
