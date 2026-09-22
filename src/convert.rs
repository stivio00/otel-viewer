//! Converts OTLP export requests into flat DuckDB rows.

use crate::proto::opentelemetry::proto as otelpb;
use crate::rows::{LogRow, MetricPointRow, SpanRow};
use otelpb::{
    collector::{
        logs::v1::ExportLogsServiceRequest, metrics::v1::ExportMetricsServiceRequest,
        trace::v1::ExportTraceServiceRequest,
    },
    common::v1::{AnyValue, KeyValue},
    metrics::v1::{
        ExponentialHistogramDataPoint, HistogramDataPoint, NumberDataPoint, SummaryDataPoint,
        exemplar, exponential_histogram_data_point::Buckets, metric::Data, number_data_point,
    },
    resource::v1::Resource,
    trace::v1::{Span, span},
};
use serde_json::{Value, json};

/// Proto nanosecond timestamps are u64; clamp to i64 for storage.
fn ns(v: u64) -> i64 {
    i64::try_from(v).unwrap_or(i64::MAX)
}

fn hex_id(b: &[u8]) -> String {
    if b.is_empty() {
        String::new()
    } else {
        hex::encode(b)
    }
}

/// Non-finite doubles are not valid JSON; render them as strings.
fn dnum(v: f64) -> Value {
    if v.is_finite() {
        json!(v)
    } else {
        json!(v.to_string())
    }
}

fn any_to_value(av: &AnyValue) -> Value {
    use otelpb::common::v1::any_value::Value as Av;
    match av.value.as_ref() {
        None => Value::Null,
        Some(v) => match v {
            Av::StringValue(s) => json!(s),
            Av::BoolValue(b) => json!(b),
            Av::IntValue(i) => json!(i),
            Av::DoubleValue(d) => dnum(*d),
            Av::ArrayValue(a) => {
                json!(a.values.iter().map(any_to_value).collect::<Vec<_>>())
            }
            Av::KvlistValue(kv) => Value::Object(kvlist_value(kv)),
            Av::BytesValue(b) => json!(hex::encode(b)),
            // String interning is only used by the (unsupported) profiles
            // signal; there is no dictionary to resolve it against here.
            Av::StringValueStrindex(_) => Value::Null,
        },
    }
}

fn kvlist_value(kv: &otelpb::common::v1::KeyValueList) -> serde_json::Map<String, Value> {
    let mut m = serde_json::Map::new();
    for pair in &kv.values {
        insert_kv(&mut m, pair);
    }
    m
}

fn insert_kv(m: &mut serde_json::Map<String, Value>, pair: &KeyValue) {
    let v = match pair.value.as_ref() {
        None => Value::Null,
        Some(av) => any_to_value(av),
    };
    m.insert(pair.key.clone(), v);
}

fn attrs_to_json(attrs: &[KeyValue]) -> Option<String> {
    if attrs.is_empty() {
        return None;
    }
    let mut m = serde_json::Map::new();
    for pair in attrs {
        insert_kv(&mut m, pair);
    }
    Some(Value::Object(m).to_string())
}

struct ResCtx {
    service_name: String,
    resource_attributes: Option<String>,
}

impl ResCtx {
    fn new(resource: Option<&Resource>) -> Self {
        let empty = Resource::default();
        let r = resource.unwrap_or(&empty);
        let mut ctx = ResCtx {
            service_name: "unknown_service".to_string(),
            resource_attributes: None,
        };
        if !r.attributes.is_empty() {
            let mut m = serde_json::Map::new();
            for pair in &r.attributes {
                insert_kv(&mut m, pair);
            }
            if let Some(v) = m.get("service.name")
                && !v.is_null()
            {
                ctx.service_name = v.to_string().trim_matches('"').to_string();
            }
            ctx.resource_attributes = Some(Value::Object(m).to_string());
        }
        ctx
    }
}

#[allow(clippy::type_complexity)]
fn scope_parts(
    scope: &Option<otelpb::common::v1::InstrumentationScope>,
) -> (Option<String>, Option<String>) {
    match scope {
        Some(s) => (
            (!s.name.is_empty()).then(|| s.name.clone()),
            opt_str(&s.version),
        ),
        None => (None, None),
    }
}

fn opt_str(s: &str) -> Option<String> {
    (!s.is_empty()).then(|| s.to_string())
}

// ---------------------------------------------------------------------------
// Traces
// ---------------------------------------------------------------------------

pub fn trace_request(req: &ExportTraceServiceRequest) -> Vec<SpanRow> {
    let mut rows = Vec::new();
    for rs in &req.resource_spans {
        let ctx = ResCtx::new(rs.resource.as_ref());
        for ss in &rs.scope_spans {
            let (scope_name, scope_version) = scope_parts(&ss.scope);
            let schema_url = opt_str(&ss.schema_url).or_else(|| opt_str(&rs.schema_url));
            for span in &ss.spans {
                rows.push(span_row(
                    span,
                    &ctx,
                    &scope_name,
                    &scope_version,
                    &schema_url,
                ));
            }
        }
    }
    rows
}

fn span_row(
    s: &Span,
    ctx: &ResCtx,
    scope_name: &Option<String>,
    scope_version: &Option<String>,
    schema_url: &Option<String>,
) -> SpanRow {
    let status = s.status.as_ref();
    SpanRow {
        trace_id: hex_id(&s.trace_id),
        span_id: hex_id(&s.span_id),
        parent_span_id: (!s.parent_span_id.is_empty()).then(|| hex_id(&s.parent_span_id)),
        trace_state: opt_str(&s.trace_state),
        span_name: s.name.clone(),
        span_kind: s.kind,
        start_ns: ns(s.start_time_unix_nano),
        end_ns: ns(s.end_time_unix_nano),
        service_name: ctx.service_name.clone(),
        scope_name: scope_name.clone(),
        scope_version: scope_version.clone(),
        schema_url: schema_url.clone(),
        resource_attributes: ctx.resource_attributes.clone(),
        span_attributes: attrs_to_json(&s.attributes),
        status_code: status.map_or(0, |st| st.code),
        status_message: status.and_then(|st| opt_str(&st.message)),
        events: events_json(&s.events),
        links: links_json(&s.links),
        dropped_attributes_count: s.dropped_attributes_count as i32,
        dropped_events_count: s.dropped_events_count as i32,
        dropped_links_count: s.dropped_links_count as i32,
        flags: s.flags as i64,
    }
}

fn events_json(events: &[span::Event]) -> Option<String> {
    if events.is_empty() {
        return None;
    }
    let v: Vec<Value> = events
        .iter()
        .map(|e| {
            json!({
                "name": e.name,
                "time_ns": ns(e.time_unix_nano).to_string(),
                "attributes": attrs_obj(&e.attributes),
                "dropped_attributes_count": e.dropped_attributes_count,
            })
        })
        .collect();
    Some(Value::Array(v).to_string())
}

fn links_json(links: &[span::Link]) -> Option<String> {
    if links.is_empty() {
        return None;
    }
    let v: Vec<Value> = links
        .iter()
        .map(|l| {
            json!({
                "trace_id": hex_id(&l.trace_id),
                "span_id": hex_id(&l.span_id),
                "trace_state": l.trace_state,
                "attributes": attrs_obj(&l.attributes),
                "dropped_attributes_count": l.dropped_attributes_count,
                "flags": l.flags,
            })
        })
        .collect();
    Some(Value::Array(v).to_string())
}

fn attrs_obj(attrs: &[KeyValue]) -> Value {
    if attrs.is_empty() {
        return json!({});
    }
    let mut m = serde_json::Map::new();
    for pair in attrs {
        insert_kv(&mut m, pair);
    }
    Value::Object(m)
}

// ---------------------------------------------------------------------------
// Logs
// ---------------------------------------------------------------------------

pub fn logs_request(req: &ExportLogsServiceRequest) -> Vec<LogRow> {
    let mut rows = Vec::new();
    for rl in &req.resource_logs {
        let ctx = ResCtx::new(rl.resource.as_ref());
        for sl in &rl.scope_logs {
            let (scope_name, scope_version) = scope_parts(&sl.scope);
            let schema_url = opt_str(&sl.schema_url).or_else(|| opt_str(&rl.schema_url));
            for rec in &sl.log_records {
                rows.push(log_row(rec, &ctx, &scope_name, &scope_version, &schema_url));
            }
        }
    }
    rows
}

fn log_row(
    r: &otelpb::logs::v1::LogRecord,
    ctx: &ResCtx,
    scope_name: &Option<String>,
    scope_version: &Option<String>,
    schema_url: &Option<String>,
) -> LogRow {
    LogRow {
        time_ns: ns(r.time_unix_nano),
        observed_time_ns: (r.observed_time_unix_nano != 0).then(|| ns(r.observed_time_unix_nano)),
        severity_text: opt_str(&r.severity_text),
        severity_number: r.severity_number,
        service_name: ctx.service_name.clone(),
        scope_name: scope_name.clone(),
        scope_version: scope_version.clone(),
        schema_url: schema_url.clone(),
        body: r.body.as_ref().map(any_to_value).map(|v| v.to_string()),
        trace_id: (!r.trace_id.is_empty()).then(|| hex_id(&r.trace_id)),
        span_id: (!r.span_id.is_empty()).then(|| hex_id(&r.span_id)),
        event_name: opt_str(&r.event_name),
        attributes: attrs_to_json(&r.attributes),
        resource_attributes: ctx.resource_attributes.clone(),
        flags: r.flags as i64,
    }
}

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

pub fn metrics_request(req: &ExportMetricsServiceRequest) -> Vec<MetricPointRow> {
    let mut rows = Vec::new();
    for rm in &req.resource_metrics {
        let ctx = ResCtx::new(rm.resource.as_ref());
        for sm in &rm.scope_metrics {
            let (scope_name, scope_version) = scope_parts(&sm.scope);
            let schema_url = opt_str(&sm.schema_url).or_else(|| opt_str(&rm.schema_url));
            for m in &sm.metrics {
                let data = match m.data.as_ref() {
                    Some(d) => d,
                    None => continue,
                };
                let metadata = attrs_to_json(&m.metadata);
                let mut push = |r: MetricPointRow| {
                    let mut r = r;
                    r.metric_metadata = metadata.clone();
                    rows.push(r);
                };
                let base = PointBase {
                    metric_name: m.name.clone(),
                    metric_unit: opt_str(&m.unit),
                    metric_description: opt_str(&m.description),
                    service_name: ctx.service_name.clone(),
                    scope_name: scope_name.clone(),
                    scope_version: scope_version.clone(),
                    schema_url: schema_url.clone(),
                    resource_attributes: ctx.resource_attributes.clone(),
                };
                match data {
                    Data::Gauge(g) => {
                        for dp in &g.data_points {
                            push(number_row(&base, "gauge", None, None, dp));
                        }
                    }
                    Data::Sum(s) => {
                        for dp in &s.data_points {
                            push(number_row(
                                &base,
                                "sum",
                                Some(s.aggregation_temporality),
                                Some(s.is_monotonic),
                                dp,
                            ));
                        }
                    }
                    Data::Histogram(h) => {
                        for dp in &h.data_points {
                            push(histogram_row(&base, Some(h.aggregation_temporality), dp));
                        }
                    }
                    Data::ExponentialHistogram(h) => {
                        for dp in &h.data_points {
                            push(exp_histogram_row(
                                &base,
                                Some(h.aggregation_temporality),
                                dp,
                            ));
                        }
                    }
                    Data::Summary(s) => {
                        for dp in &s.data_points {
                            push(summary_row(&base, dp));
                        }
                    }
                }
            }
        }
    }
    rows
}

struct PointBase {
    metric_name: String,
    metric_unit: Option<String>,
    metric_description: Option<String>,
    service_name: String,
    scope_name: Option<String>,
    scope_version: Option<String>,
    schema_url: Option<String>,
    resource_attributes: Option<String>,
}

fn number_row(
    base: &PointBase,
    metric_type: &str,
    temporality: Option<i32>,
    is_monotonic: Option<bool>,
    dp: &NumberDataPoint,
) -> MetricPointRow {
    let (value_double, value_int) = match dp.value.as_ref() {
        Some(number_data_point::Value::AsDouble(d)) => (Some(*d), None),
        Some(number_data_point::Value::AsInt(i)) => (None, Some(*i)),
        None => (None, None),
    };
    MetricPointRow {
        metric_name: base.metric_name.clone(),
        metric_type: metric_type.to_string(),
        metric_unit: base.metric_unit.clone(),
        metric_description: base.metric_description.clone(),
        aggregation_temporality: temporality,
        is_monotonic,
        service_name: base.service_name.clone(),
        scope_name: base.scope_name.clone(),
        scope_version: base.scope_version.clone(),
        schema_url: base.schema_url.clone(),
        resource_attributes: base.resource_attributes.clone(),
        series_attributes: attrs_to_json(&dp.attributes),
        metric_metadata: None,
        exemplars: exemplars_json(&dp.exemplars),
        start_ns: (dp.start_time_unix_nano != 0).then(|| ns(dp.start_time_unix_nano)),
        ts_ns: ns(dp.time_unix_nano),
        value_double,
        value_int,
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
        flags: dp.flags as i64,
    }
}

fn histogram_row(
    base: &PointBase,
    temporality: Option<i32>,
    dp: &HistogramDataPoint,
) -> MetricPointRow {
    MetricPointRow {
        metric_name: base.metric_name.clone(),
        metric_type: "histogram".to_string(),
        metric_unit: base.metric_unit.clone(),
        metric_description: base.metric_description.clone(),
        aggregation_temporality: temporality,
        is_monotonic: None,
        service_name: base.service_name.clone(),
        scope_name: base.scope_name.clone(),
        scope_version: base.scope_version.clone(),
        schema_url: base.schema_url.clone(),
        resource_attributes: base.resource_attributes.clone(),
        series_attributes: attrs_to_json(&dp.attributes),
        metric_metadata: None,
        exemplars: exemplars_json(&dp.exemplars),
        start_ns: (dp.start_time_unix_nano != 0).then(|| ns(dp.start_time_unix_nano)),
        ts_ns: ns(dp.time_unix_nano),
        value_double: None,
        value_int: None,
        hist_count: Some(dp.count as i64),
        hist_sum: dp.sum,
        hist_min: dp.min,
        hist_max: dp.max,
        hist_bounds: json_arr(
            &dp.explicit_bounds
                .iter()
                .map(|b| dnum(*b))
                .collect::<Vec<_>>(),
        ),
        hist_bucket_counts: json_arr(
            &dp.bucket_counts
                .iter()
                .map(|c| json!(c))
                .collect::<Vec<_>>(),
        ),
        exp_zero_count: None,
        exp_scale: None,
        exp_zero_threshold: None,
        exp_buckets: None,
        summary_count: None,
        summary_sum: None,
        summary_quantiles: None,
        flags: dp.flags as i64,
    }
}

fn exp_histogram_row(
    base: &PointBase,
    temporality: Option<i32>,
    dp: &ExponentialHistogramDataPoint,
) -> MetricPointRow {
    let buckets_json = |b: &Option<Buckets>| {
        b.as_ref().map(|b| {
            json!({
                "offset": b.offset,
                "bucket_counts": b.bucket_counts,
            })
            .to_string()
        })
    };
    MetricPointRow {
        metric_name: base.metric_name.clone(),
        metric_type: "exponential_histogram".to_string(),
        metric_unit: base.metric_unit.clone(),
        metric_description: base.metric_description.clone(),
        aggregation_temporality: temporality,
        is_monotonic: None,
        service_name: base.service_name.clone(),
        scope_name: base.scope_name.clone(),
        scope_version: base.scope_version.clone(),
        schema_url: base.schema_url.clone(),
        resource_attributes: base.resource_attributes.clone(),
        series_attributes: attrs_to_json(&dp.attributes),
        metric_metadata: None,
        exemplars: exemplars_json(&dp.exemplars),
        start_ns: (dp.start_time_unix_nano != 0).then(|| ns(dp.start_time_unix_nano)),
        ts_ns: ns(dp.time_unix_nano),
        value_double: None,
        value_int: None,
        hist_count: Some(dp.count as i64),
        hist_sum: dp.sum,
        hist_min: dp.min,
        hist_max: dp.max,
        hist_bounds: None,
        hist_bucket_counts: None,
        exp_zero_count: Some(dp.zero_count as i64),
        exp_scale: Some(dp.scale),
        exp_zero_threshold: Some(dp.zero_threshold),
        exp_buckets: Some(
            json!({
                "positive": opt_json_str(&buckets_json(&dp.positive)),
                "negative": opt_json_str(&buckets_json(&dp.negative)),
            })
            .to_string(),
        ),
        summary_count: None,
        summary_sum: None,
        summary_quantiles: None,
        flags: dp.flags as i64,
    }
}

fn summary_row(base: &PointBase, dp: &SummaryDataPoint) -> MetricPointRow {
    MetricPointRow {
        metric_name: base.metric_name.clone(),
        metric_type: "summary".to_string(),
        metric_unit: base.metric_unit.clone(),
        metric_description: base.metric_description.clone(),
        aggregation_temporality: None,
        is_monotonic: None,
        service_name: base.service_name.clone(),
        scope_name: base.scope_name.clone(),
        scope_version: base.scope_version.clone(),
        schema_url: base.schema_url.clone(),
        resource_attributes: base.resource_attributes.clone(),
        series_attributes: attrs_to_json(&dp.attributes),
        metric_metadata: None,
        exemplars: None,
        start_ns: (dp.start_time_unix_nano != 0).then(|| ns(dp.start_time_unix_nano)),
        ts_ns: ns(dp.time_unix_nano),
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
        summary_count: Some(dp.count as i64),
        summary_sum: Some(dp.sum),
        summary_quantiles: json_arr(
            &dp.quantile_values
                .iter()
                .map(|q| json!({"quantile": q.quantile, "value": dnum(q.value)}))
                .collect::<Vec<_>>(),
        ),
        flags: 0,
    }
}

fn exemplars_json(exemplars: &[otelpb::metrics::v1::Exemplar]) -> Option<String> {
    if exemplars.is_empty() {
        return None;
    }
    let v: Vec<Value> = exemplars
        .iter()
        .map(|e| {
            let value = match e.value.as_ref() {
                Some(exemplar::Value::AsDouble(d)) => dnum(*d),
                Some(exemplar::Value::AsInt(i)) => json!(i),
                None => Value::Null,
            };
            json!({
                "value": value,
                "time_ns": ns(e.time_unix_nano).to_string(),
                "trace_id": hex_id(&e.trace_id),
                "span_id": hex_id(&e.span_id),
                "filtered_attributes": attrs_obj(&e.filtered_attributes),
            })
        })
        .collect();
    Some(Value::Array(v).to_string())
}

fn json_arr(v: &[Value]) -> Option<String> {
    (!v.is_empty()).then(|| Value::Array(v.to_vec()).to_string())
}

fn opt_json_str(s: &Option<String>) -> Value {
    s.as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(Value::Null)
}
