export interface Health {
  status: string
  version: string
  db_file: string | null
  otlp_addr: string | null
}

export interface Stats {
  spans: number
  traces: number
  logs: number
  metric_points: number
  metrics: number
  services: number
  earliest_ns: string | null
  latest_ns: string | null
  db_file: string | null
}

export interface TableStats {
  table_name: string
  estimated_size: number | null
  column_count: number | null
  index_count: number | null
}

export interface DbStats {
  db_file: string | null
  file_size_bytes: number | null
  database_size: string | null
  block_size: number | null
  total_blocks: number | null
  used_blocks: number | null
  free_blocks: number | null
  checkpoint_count: number | null
  memory_bytes: number | null
  tables: TableStats[]
}

export interface ServiceInfo {
  name: string
  first_ns: string | null
  last_ns: string | null
  span_count: number
  log_count: number
  point_count: number
}

export interface ColumnInfo {
  name: string
  type: string
}

export interface TableInfo {
  name: string
  row_count: number
  columns: ColumnInfo[]
}

export interface Schema {
  tables: TableInfo[]
}

export interface TraceSummary {
  trace_id: string
  root_name: string | null
  service: string | null
  services: string[]
  start_ns: string
  duration_ns: string
  span_count: number
  error_count: number
}

export interface TracesResponse {
  total: number
  traces: TraceSummary[]
}

export interface SpanDto {
  trace_id: string
  span_id: string
  parent_span_id: string | null
  trace_state: string | null
  span_name: string
  span_kind: number
  start_ns: string
  end_ns: string
  duration_ms: number
  service_name: string
  scope_name: string | null
  scope_version: string | null
  schema_url: string | null
  resource_attributes: unknown
  attributes: unknown
  status_code: number
  status_message: string | null
  events: unknown
  links: unknown
  dropped_attributes_count: number
  dropped_events_count: number
  dropped_links_count: number
  flags: number
}

export interface TraceDetail {
  trace_id: string
  start_ns: string
  duration_ns: string
  span_count: number
  error_count: number
  services: string[]
  spans: SpanDto[]
}

export interface LogDto {
  time_ns: string
  observed_time_ns: string | null
  severity_text: string | null
  severity_number: number
  service_name: string
  scope_name: string | null
  scope_version: string | null
  schema_url: string | null
  body: unknown
  trace_id: string | null
  span_id: string | null
  event_name: string | null
  attributes: unknown
  resource_attributes: unknown
  flags: number
}

export interface LogsResponse {
  total: number
  logs: LogDto[]
}

export interface MetricSummary {
  name: string
  metric_type: string
  unit: string | null
  description: string | null
  point_count: number
  service_count: number
  services: string[]
  first_ns: string
  last_ns: string
  last_value: number | null
}

export interface MetricsResponse {
  metrics: MetricSummary[]
}

export interface MetricPointDto {
  service_name: string
  scope_name: string | null
  series_attributes: unknown
  resource_attributes: unknown
  exemplars: unknown
  metric_metadata: unknown
  aggregation_temporality: number | null
  is_monotonic: boolean | null
  start_ns: string | null
  ts_ns: string
  value_double: number | null
  value_int: number | null
  hist_count: number | null
  hist_sum: number | null
  hist_min: number | null
  hist_max: number | null
  hist_bounds: unknown
  hist_bucket_counts: unknown
  exp_zero_count: number | null
  exp_scale: number | null
  exp_zero_threshold: number | null
  exp_buckets: unknown
  summary_count: number | null
  summary_sum: number | null
  summary_quantiles: unknown
}

export interface MetricDetail {
  name: string
  metric_type: string
  unit: string | null
  description: string | null
  points: MetricPointDto[]
}

export interface QueryResponse {
  columns: ColumnInfo[]
  rows: Array<Record<string, unknown>>
  row_count: number
  truncated: boolean
  elapsed_ms: number
}

async function get<T>(path: string): Promise<T> {
  const res = await fetch(path)
  if (!res.ok) {
    const body = await res.text().catch(() => "")
    throw new Error(`${res.status}: ${body || res.statusText}`)
  }
  return res.json() as Promise<T>
}

export interface TraceFilters {
  service?: string
  q?: string
  start_ns?: string
  end_ns?: string
  min_duration_ms?: number
  errors_only?: boolean
  limit?: number
  offset?: number
}

export function fetchTraces(f: TraceFilters): Promise<TracesResponse> {
  const p = new URLSearchParams()
  if (f.service) p.set("service", f.service)
  if (f.q) p.set("q", f.q)
  if (f.start_ns) p.set("start_ns", f.start_ns)
  if (f.end_ns) p.set("end_ns", f.end_ns)
  if (f.min_duration_ms != null) p.set("min_duration_ms", String(f.min_duration_ms))
  if (f.errors_only) p.set("errors_only", "true")
  if (f.limit != null) p.set("limit", String(f.limit))
  if (f.offset != null) p.set("offset", String(f.offset))
  return get(`/api/traces?${p}`)
}

export function fetchTraceDetail(traceId: string): Promise<TraceDetail> {
  return get(`/api/traces/${traceId}`)
}

export interface LogFilters {
  service?: string
  q?: string
  severity?: string
  start_ns?: string
  end_ns?: string
  trace_id?: string
  span_id?: string
  limit?: number
  offset?: number
}

export function fetchLogs(f: LogFilters): Promise<LogsResponse> {
  const p = new URLSearchParams()
  if (f.service) p.set("service", f.service)
  if (f.q) p.set("q", f.q)
  if (f.severity) p.set("severity", f.severity)
  if (f.start_ns) p.set("start_ns", f.start_ns)
  if (f.end_ns) p.set("end_ns", f.end_ns)
  if (f.trace_id) p.set("trace_id", f.trace_id)
  if (f.span_id) p.set("span_id", f.span_id)
  if (f.limit != null) p.set("limit", String(f.limit))
  if (f.offset != null) p.set("offset", String(f.offset))
  return get(`/api/logs?${p}`)
}

export function fetchMetrics(q?: string, limit?: number): Promise<MetricsResponse> {
  const p = new URLSearchParams()
  if (q) p.set("q", q)
  if (limit != null) p.set("limit", String(limit))
  return get(`/api/metrics?${p}`)
}

export function fetchMetricDetail(
  name: string,
  f: { service?: string; start_ns?: string; end_ns?: string; limit?: number } = {}
): Promise<MetricDetail> {
  const p = new URLSearchParams()
  if (f.service) p.set("service", f.service)
  if (f.start_ns) p.set("start_ns", f.start_ns)
  if (f.end_ns) p.set("end_ns", f.end_ns)
  if (f.limit != null) p.set("limit", String(f.limit))
  return get(`/api/metrics/${encodeURIComponent(name)}?${p}`)
}

export function runQuery(sql: string, limit = 1000): Promise<QueryResponse> {
  return fetch("/api/query", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ sql, limit }),
  }).then(async (res) => {
    if (!res.ok) {
      const body = await res.json().catch(() => ({}))
      throw new Error(body.error ?? `${res.status} ${res.statusText}`)
    }
    return res.json() as Promise<QueryResponse>
  })
}

export const fetchHealth = () => get<Health>("/api/health")
export const fetchStats = () => get<Stats>("/api/stats")
export const fetchDbStats = () => get<DbStats>("/api/dbstats")
export const fetchServices = () => get<{ services: ServiceInfo[] }>("/api/services")
export const fetchSchema = () => get<Schema>("/api/schema")

export async function resetDb(): Promise<void> {
  const res = await fetch("/api/reset", { method: "POST" })
  if (!res.ok) {
    const body = await res.text().catch(() => "")
    throw new Error(`${res.status}: ${body || res.statusText}`)
  }
}
