//! End-to-end test: start the collector on ephemeral ports, push OTLP data
//! over gRPC (as a real exporter would), and verify it via the REST API.

use std::net::TcpListener;
use std::time::Duration;

use otel_viewer::proto::opentelemetry::proto::{
    collector::{
        logs::v1::{ExportLogsServiceRequest, logs_service_client::LogsServiceClient},
        metrics::v1::{ExportMetricsServiceRequest, metrics_service_client::MetricsServiceClient},
        trace::v1::{ExportTraceServiceRequest, trace_service_client::TraceServiceClient},
    },
    common::v1::{AnyValue, KeyValue, any_value::Value as AnyVal},
    logs::v1::{LogRecord, ResourceLogs, ScopeLogs},
    metrics::v1::{
        Metric, NumberDataPoint, ResourceMetrics, ScopeMetrics, Sum, metric::Data,
        number_data_point::Value as NumVal,
    },
    resource::v1::Resource,
    trace::v1::{ResourceSpans, ScopeSpans, Span, Status},
};
use tokio_util::sync::CancellationToken;

fn kv(k: &str, v: &str) -> KeyValue {
    KeyValue {
        key: k.to_string(),
        value: Some(AnyValue {
            value: Some(AnyVal::StringValue(v.to_string())),
        }),
        key_strindex: 0,
    }
}

fn scope(
    name: &str,
    version: Option<&str>,
) -> otel_viewer::proto::opentelemetry::proto::common::v1::InstrumentationScope {
    otel_viewer::proto::opentelemetry::proto::common::v1::InstrumentationScope {
        name: name.to_string(),
        version: version.unwrap_or_default().to_string(),
        attributes: vec![],
        dropped_attributes_count: 0,
    }
}

fn resource(service: &str) -> Resource {
    Resource {
        attributes: vec![kv("service.name", service)],
        dropped_attributes_count: 0,
        entity_refs: vec![],
    }
}

fn trace_id(i: u8) -> Vec<u8> {
    vec![i; 16]
}

fn span_id(i: u8) -> Vec<u8> {
    vec![i; 8]
}

#[tokio::test]
async fn end_to_end() -> anyhow::Result<()> {
    // Bind ephemeral ports, then hand the listeners to the app.
    let http_listener = TcpListener::bind("127.0.0.1:0")?;
    let grpc_listener = TcpListener::bind("127.0.0.1:0")?;
    let http_port = http_listener.local_addr()?.port();
    let grpc_port = grpc_listener.local_addr()?.port();

    let shutdown = CancellationToken::new();
    let handle = {
        let cfg = otel_viewer::Config {
            db_file: None,
            seed_demo: false,
        };
        let shutdown = shutdown.clone();
        tokio::spawn(
            async move { otel_viewer::run(&cfg, http_listener, grpc_listener, shutdown).await },
        )
    };
    tokio::time::sleep(Duration::from_millis(300)).await;

    // ------------------------------------------------------------------
    // Push a trace over gRPC, exactly like an OTLP exporter.
    // ------------------------------------------------------------------
    let channel = tonic::transport::Channel::from_shared(format!("http://127.0.0.1:{grpc_port}"))?
        .connect()
        .await?;

    let now = 1_700_000_000_000_000_000u64;
    let trace = ExportTraceServiceRequest {
        resource_spans: vec![ResourceSpans {
            resource: Some(resource("e2e-service")),
            scope_spans: vec![ScopeSpans {
                scope: Some(scope("e2e-scope", Some("0.1"))),
                spans: vec![
                    Span {
                        trace_id: trace_id(1),
                        span_id: span_id(1),
                        parent_span_id: vec![],
                        trace_state: "key=value".to_string(),
                        flags: 1,
                        name: "GET /users".to_string(),
                        kind: 2, // SERVER
                        start_time_unix_nano: now,
                        end_time_unix_nano: now + 50_000_000,
                        attributes: vec![kv("http.route", "/users")],
                        dropped_attributes_count: 0,
                        events: vec![
                            otel_viewer::proto::opentelemetry::proto::trace::v1::span::Event {
                                time_unix_nano: now + 1_000_000,
                                name: "cache.miss".to_string(),
                                attributes: vec![kv("key", "user:1")],
                                dropped_attributes_count: 0,
                            },
                        ],
                        dropped_events_count: 0,
                        links: vec![],
                        dropped_links_count: 0,
                        status: Some(Status {
                            message: String::new(),
                            code: 1, // OK
                        }),
                    },
                    Span {
                        trace_id: trace_id(1),
                        span_id: span_id(2),
                        parent_span_id: span_id(1),
                        trace_state: String::new(),
                        flags: 1,
                        name: "SELECT users".to_string(),
                        kind: 3, // CLIENT
                        start_time_unix_nano: now + 5_000_000,
                        end_time_unix_nano: now + 40_000_000,
                        attributes: vec![kv("db.system", "postgresql")],
                        dropped_attributes_count: 0,
                        events: vec![],
                        dropped_events_count: 0,
                        links: vec![],
                        dropped_links_count: 0,
                        status: Some(Status {
                            message: "connection refused".to_string(),
                            code: 2, // ERROR
                        }),
                    },
                ],
                schema_url: "https://example.com/schema".to_string(),
            }],
            schema_url: String::new(),
        }],
    };
    TraceServiceClient::new(channel.clone())
        .export(trace)
        .await?;

    // ------------------------------------------------------------------
    // Push logs (one linked to the trace, one standalone).
    // ------------------------------------------------------------------
    let logs_req = ExportLogsServiceRequest {
        resource_logs: vec![ResourceLogs {
            resource: Some(resource("e2e-service")),
            scope_logs: vec![ScopeLogs {
                scope: Some(scope("e2e-scope", None)),
                log_records: vec![
                    LogRecord {
                        time_unix_nano: now + 10_000_000,
                        observed_time_unix_nano: now + 11_000_000,
                        severity_number: 9, // INFO
                        severity_text: "INFO".to_string(),
                        body: Some(AnyValue {
                            value: Some(AnyVal::StringValue("user query succeeded".to_string())),
                        }),
                        attributes: vec![kv("user.id", "42")],
                        dropped_attributes_count: 0,
                        flags: 0,
                        trace_id: trace_id(1),
                        span_id: span_id(1),
                        event_name: String::new(),
                    },
                    LogRecord {
                        time_unix_nano: now + 45_000_000,
                        observed_time_unix_nano: 0,
                        severity_number: 17, // ERROR
                        severity_text: "ERROR".to_string(),
                        body: Some(AnyValue {
                            value: Some(AnyVal::StringValue("db connection refused".to_string())),
                        }),
                        attributes: vec![kv("db.system", "postgresql")],
                        dropped_attributes_count: 0,
                        flags: 0,
                        trace_id: trace_id(1),
                        span_id: span_id(2),
                        event_name: String::new(),
                    },
                ],
                schema_url: String::new(),
            }],
            schema_url: String::new(),
        }],
    };
    LogsServiceClient::new(channel.clone())
        .export(logs_req)
        .await?;

    // ------------------------------------------------------------------
    // Push metrics (sum + histogram).
    // ------------------------------------------------------------------
    let metrics_req = ExportMetricsServiceRequest {
        resource_metrics: vec![ResourceMetrics {
            resource: Some(resource("e2e-service")),
            scope_metrics: vec![ScopeMetrics {
                scope: Some(scope("e2e-scope", None)),
                metrics: vec![
                    Metric {
                        name: "requests.total".to_string(),
                        description: "total requests".to_string(),
                        unit: "{requests}".to_string(),
                        data: Some(Data::Sum(Sum {
                            data_points: vec![NumberDataPoint {
                                attributes: vec![kv("route", "/users")],
                                start_time_unix_nano: now,
                                time_unix_nano: now + 60_000_000,
                                value: Some(NumVal::AsDouble(42.5)),
                                exemplars: vec![],
                                flags: 0,
                            }],
                            aggregation_temporality: 2, // CUMULATIVE
                            is_monotonic: true,
                        })),
                        metadata: vec![],
                    },
                    Metric {
                        name: "latency.ms".to_string(),
                        description: String::new(),
                        unit: "ms".to_string(),
                        data: Some(Data::Histogram(otel_viewer::proto::opentelemetry::proto::metrics::v1::Histogram {
                            data_points: vec![otel_viewer::proto::opentelemetry::proto::metrics::v1::HistogramDataPoint {
                                attributes: vec![],
                                start_time_unix_nano: now,
                                time_unix_nano: now + 60_000_000,
                                count: 7,
                                sum: Some(123.0),
                                bucket_counts: vec![1, 2, 4],
                                explicit_bounds: vec![5.0, 10.0],
                                exemplars: vec![],
                                flags: 0,
                                min: Some(1.0),
                                max: Some(30.0),
                            }],
                            aggregation_temporality: 2,
                        })),
                        metadata: vec![],
                    },
                ],
                schema_url: String::new(),
            }],
            schema_url: String::new(),
        }],
    };
    MetricsServiceClient::new(channel)
        .export(metrics_req)
        .await?;

    // Give the writer a moment, then hit the REST API.
    tokio::time::sleep(Duration::from_millis(200)).await;
    let http = format!("http://127.0.0.1:{http_port}");
    let client = reqwest::Client::new();

    // -- stats ---------------------------------------------------------
    let stats: serde_json::Value = client
        .get(format!("{http}/api/stats"))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(stats["spans"], 2);
    assert_eq!(stats["traces"], 1);
    assert_eq!(stats["logs"], 2);
    assert_eq!(stats["metric_points"], 2);
    assert_eq!(stats["services"], 1);

    // -- traces list ----------------------------------------------------
    let traces: serde_json::Value = client
        .get(format!(
            "{http}/api/traces?service=e2e-service&errors_only=true"
        ))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(traces["total"], 1, "errors_only filter should match");
    assert_eq!(traces["traces"][0]["span_count"], 2);
    assert_eq!(traces["traces"][0]["error_count"], 1);
    assert_eq!(traces["traces"][0]["root_name"], "GET /users");

    // no errors_only -> still 1 trace; filters by name search
    let all: serde_json::Value = client
        .get(format!("{http}/api/traces?q=users"))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(all["total"], 1);
    let none: serde_json::Value = client
        .get(format!("{http}/api/traces?q=nonexistent"))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(none["total"], 0);

    // -- trace detail ----------------------------------------------------
    let detail: serde_json::Value = client
        .get(format!("{http}/api/traces/{}", hex_str(&trace_id(1))))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(detail["spans"].as_array().map(Vec::len), Some(2));
    assert_eq!(detail["spans"][0]["span_name"], "GET /users");
    assert_eq!(detail["spans"][1]["parent_span_id"], hex_str(&span_id(1)));
    assert_eq!(
        detail["spans"][0]["attributes"]["http.route"], "/users",
        "span attributes should be parsed JSON"
    );
    assert_eq!(detail["spans"][0]["events"][0]["name"], "cache.miss");

    // 404 for unknown trace
    let resp = client
        .get(format!("{http}/api/traces/deadbeef"))
        .send()
        .await?;
    assert_eq!(resp.status(), reqwest::StatusCode::NOT_FOUND);

    // -- logs -------------------------------------------------------------
    let logs: serde_json::Value = client
        .get(format!("{http}/api/logs?severity=error"))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(logs["total"], 1);
    assert_eq!(logs["logs"][0]["body"], "db connection refused");
    assert_eq!(logs["logs"][0]["trace_id"], hex_str(&trace_id(1)));

    let log_search: serde_json::Value = client
        .get(format!("{http}/api/logs?q=succeeded"))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(log_search["total"], 1);

    let trace_logs: serde_json::Value = client
        .get(format!(
            "{http}/api/logs?trace_id={}",
            hex_str(&trace_id(1))
        ))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(trace_logs["total"], 2);

    // -- metrics ------------------------------------------------------------
    let metrics: serde_json::Value = client
        .get(format!("{http}/api/metrics?q=latency"))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(metrics["metrics"].as_array().map(Vec::len), Some(1));
    assert_eq!(metrics["metrics"][0]["metric_type"], "histogram");

    let mdetail: serde_json::Value = client
        .get(format!("{http}/api/metrics/latency.ms"))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(mdetail["points"].as_array().map(Vec::len), Some(1));
    assert_eq!(mdetail["points"][0]["hist_count"], 7);
    assert_eq!(mdetail["points"][0]["hist_sum"], 123.0);

    // -- services & schema ---------------------------------------------------
    let services: serde_json::Value = client
        .get(format!("{http}/api/services"))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(services["services"][0]["name"], "e2e-service");
    assert_eq!(services["services"][0]["span_count"], 2);

    let schema: serde_json::Value = client
        .get(format!("{http}/api/schema"))
        .send()
        .await?
        .json()
        .await?;
    let tables: Vec<String> = schema["tables"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    assert!(tables.contains(&"spans".to_string()));
    assert!(tables.contains(&"log_records".to_string()));

    // -- arbitrary SQL console ------------------------------------------------
    let q: serde_json::Value = client
        .post(format!("{http}/api/query"))
        .json(&serde_json::json!({"sql": "SELECT service_name, count(*) AS n FROM spans GROUP BY service_name"}))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(q["rows"][0]["service_name"], "e2e-service");
    assert_eq!(q["rows"][0]["n"], 2);

    // writes must be rejected
    let bad = client
        .post(format!("{http}/api/query"))
        .json(&serde_json::json!({"sql": "DELETE FROM spans"}))
        .send()
        .await?;
    assert_eq!(bad.status(), reqwest::StatusCode::BAD_REQUEST);

    // auto limit applied
    let limited: serde_json::Value = client
        .post(format!("{http}/api/query"))
        .json(
            &serde_json::json!({"sql": "SELECT 1 AS x FROM generate_series(1, 100)", "limit": 10}),
        )
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(limited["row_count"], 10);
    assert_eq!(limited["truncated"], true);

    // -- static UI ------------------------------------------------------------
    let index = client.get(format!("{http}/")).send().await?;
    assert!(index.status().is_success());
    let body = index.text().await?;
    assert!(body.contains("<"), "index should be html");

    shutdown.cancel();
    let _ = tokio::time::timeout(Duration::from_secs(5), handle).await;
    Ok(())
}

fn hex_str(b: &[u8]) -> String {
    hex::encode(b)
}
