import { useMemo, useState } from "react"
import { useMutation } from "@tanstack/react-query"
import CodeMirror from "@uiw/react-codemirror"
import { sql } from "@codemirror/lang-sql"
import { oneDark } from "@codemirror/theme-one-dark"
import { CartesianGrid, Line, LineChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts"
import { LineChart as LineChartIcon, Play, Table2, Terminal } from "lucide-react"

import { runQuery, type ColumnInfo, type QueryResponse } from "@/lib/api"
import { fmtNsTime, num, serviceColor } from "@/lib/format"
import { useUi } from "@/lib/store"
import { Panel, PanelGroup } from "@/components/Split"
import { EmptyState, Panel as PanelChrome } from "@/components/Panel"
import { ResizeHandle } from "@/components/Split"
import { SchemaPanel } from "@/components/panels/SchemaPanel"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { ScrollArea } from "@/components/ui/scroll-area"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"

const EXAMPLES: Array<{ label: string; sql: string }> = [
  {
    label: "Top services by span count",
    sql: "SELECT service_name, count(*) AS spans, round(avg(duration_ms), 2) AS avg_ms\nFROM spans GROUP BY 1 ORDER BY spans DESC",
  },
  {
    label: "p95 latency by service",
    sql: "SELECT service_name, round(quantile_cont(duration_ms, 0.95), 2) AS p95_ms\nFROM spans GROUP BY 1 ORDER BY p95_ms DESC",
  },
  {
    label: "Error traces",
    sql: "SELECT trace_id, span_name, status_message, duration_ms\nFROM spans WHERE status_code = 2 ORDER BY start_ns DESC LIMIT 100",
  },
  {
    label: "Slowest spans",
    sql: "SELECT span_name, service_name, duration_ms, span_attributes\nFROM spans ORDER BY duration_ms DESC LIMIT 50",
  },
  {
    label: "Logs by severity",
    sql: "SELECT severity_text, count(*) AS n\nFROM log_records GROUP BY 1 ORDER BY n DESC",
  },
  {
    label: "Recent error logs",
    sql: "SELECT to_timestamp(time_ns / 1e9) AS ts, service_name, body\nFROM log_records WHERE severity_number >= 17\nORDER BY time_ns DESC LIMIT 100",
  },
  {
    label: "Gauge values over time",
    sql: "SELECT metric_name, service_name, ts_ns / 1e9 AS ts, value_double\nFROM metric_points WHERE metric_type = 'gauge'\nORDER BY ts_ns DESC LIMIT 500",
  },
  {
    label: "Histogram means",
    sql: "SELECT metric_name, service_name,\n  round(hist_sum / hist_count, 3) AS mean\nFROM metric_points WHERE hist_count > 0\nORDER BY ts_ns DESC LIMIT 200",
  },
  {
    label: "Locust: users over time",
    sql: "SELECT to_timestamp(ts_ns / 1e9) AS ts, coalesce(value_double, value_int) AS users\nFROM metric_points\nWHERE metric_name = 'locust.current_users'\nORDER BY ts_ns",
  },
  {
    label: "Locust: latency p95 vs time",
    sql: "-- aiolocust exports cumulative histograms: take the latest snapshot\n-- per 30s bucket and read p95 off its bucket counts.\nWITH pts AS (\n  SELECT time_bucket(INTERVAL '30 seconds', to_timestamp(ts_ns / 1e9)) AS bucket,\n         ts_ns, hist_bounds::DOUBLE[] AS bounds, hist_bucket_counts::DOUBLE[] AS counts\n  FROM metric_points\n  WHERE metric_name = 'locust.client.duration' AND hist_count > 0\n),\nlast AS (\n  SELECT bucket, bounds, counts\n  FROM (SELECT *, row_number() OVER (PARTITION BY bucket ORDER BY ts_ns DESC) AS rn FROM pts)\n  WHERE rn = 1\n),\nflat AS (\n  SELECT bucket AS ts, unnest(counts) AS c,\n         unnest(list_append(bounds, 'inf'::DOUBLE)) AS bound\n  FROM last\n),\ncum AS (\n  SELECT ts, bound,\n         sum(c) OVER (PARTITION BY ts ORDER BY bound ROWS UNBOUNDED PRECEDING) AS cum_n\n  FROM flat\n),\ntot AS (\n  SELECT ts, sum(c) AS n FROM flat GROUP BY ts\n)\nSELECT t.ts AS ts,\n       round(min(c.bound) FILTER (WHERE c.cum_n >= 0.95 * t.n), 3) AS p95_s\nFROM cum c JOIN tot t USING (ts)\nGROUP BY t.ts\nORDER BY t.ts",
  },
  {
    label: "Locust: requests per second",
    sql: "-- Cumulative counters: rate = delta of hist_count between consecutive\n-- exports divided by the time between them, summed per endpoint series.\nWITH pts AS (\n  SELECT to_timestamp(ts_ns / 1e9) AS ts, ts_ns,\n         series_attributes AS series, hist_count AS cnt\n  FROM metric_points\n  WHERE metric_name = 'locust.client.duration' AND hist_count > 0\n),\nd AS (\n  SELECT ts,\n         (cnt - lag(cnt) OVER (PARTITION BY series ORDER BY ts_ns)) /\n           nullif((ts_ns - lag(ts_ns) OVER (PARTITION BY series ORDER BY ts_ns)) / 1e9, 0) AS rps\n  FROM pts\n)\nSELECT ts, round(sum(rps), 2) AS rps\nFROM d\nWHERE rps IS NOT NULL AND rps >= 0\nGROUP BY ts\nORDER BY ts",
  },
]

// ---------------------------------------------------------------------------
// Auto-detect plottable results (a time-ish column + numeric columns) and
// render them as a line chart above the table.
// ---------------------------------------------------------------------------

/** Value to epoch-ms when it looks like a timestamp, else null. */
function toTimeMs(v: unknown): number | null {
  if (typeof v === "number" && Number.isFinite(v)) {
    if (v >= 1e17) return v / 1e6 // ns
    if (v >= 1e14) return v / 1e3 // us
    if (v >= 1e11) return v // ms
    if (v >= 1e8) return v * 1e3 // s
    return null
  }
  if (typeof v === "string" && /[-:T ]/.test(v)) {
    const t = Date.parse(v)
    return Number.isNaN(t) ? null : t
  }
  return null
}

const TIME_NAME = /^(ts|t_|_?ts$|time|timestamp|when|date)/i

interface PlotShape {
  xCol: string
  series: string[]
  rows: Array<Record<string, number | null>>
}

function detectPlot(result: QueryResponse): PlotShape | null {
  const sample = result.rows.slice(0, 50)
  if (sample.length < 2) return null

  const parses = (c: ColumnInfo) => {
    let nonNull = 0
    for (const r of sample) {
      const v = r[c.name]
      if (v == null) continue
      if (toTimeMs(v) == null) return false
      nonNull++
    }
    return nonNull > 0
  }

  const xCol =
    result.columns.find((c) => TIME_NAME.test(c.name) && parses(c)) ??
    result.columns.find((c) => parses(c))
  if (!xCol) return null

  const numericCols = result.columns.filter((c) => {
    if (c.name === xCol.name) return false
    let nonNull = 0
    for (const r of sample) {
      const v = r[c.name]
      if (v == null) continue
      if (typeof v !== "number" || !Number.isFinite(v)) return false
      nonNull++
    }
    return nonNull > 0
  })
  // Skip columns that are themselves epoch timestamps (e.g. start_ns/end_ns
  // in SELECT * FROM spans) — they are never useful as y-values.
  const series = numericCols
    .filter((c) => !parses(c))
    .map((c) => c.name)
    .slice(0, 8)
  if (series.length === 0) return null

  const rows: Array<Record<string, number | null>> = []
  for (const r of result.rows) {
    const x = toTimeMs(r[xCol.name])
    if (x == null) continue
    const row: Record<string, number | null> = { x }
    for (const name of series) {
      const v = r[name]
      row[name] = typeof v === "number" && Number.isFinite(v) ? v : null
    }
    rows.push(row)
  }
  if (rows.length < 2) return null
  rows.sort((a, b) => (a.x ?? 0) - (b.x ?? 0))
  return { xCol: xCol.name, series, rows }
}

function ResultChart({ plot }: { plot: PlotShape }) {
  return (
    <div className="h-[220px] shrink-0 border-b">
      <ResponsiveContainer width="100%" height="100%">
        <LineChart data={plot.rows} margin={{ top: 12, right: 12, bottom: 0, left: 0 }}>
          <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" />
          <XAxis
            dataKey="x"
            type="number"
            scale="time"
            domain={["dataMin", "dataMax"]}
            tickFormatter={(t) => fmtNsTime(t * 1e6)}
            stroke="var(--muted-foreground)"
            fontSize={10}
            tickMargin={4}
          />
          <YAxis stroke="var(--muted-foreground)" fontSize={10} width={48} />
          <Tooltip
            contentStyle={{
              backgroundColor: "var(--popover)",
              border: "1px solid var(--border)",
              borderRadius: 8,
              fontSize: 11,
            }}
            labelFormatter={(t) => fmtNsTime(Number(t) * 1e6)}
          />
          {plot.series.map((name) => (
            <Line
              key={name}
              dataKey={name}
              name={name}
              type="monotone"
              dot={false}
              strokeWidth={1.8}
              connectNulls
              stroke={serviceColor(name)}
            />
          ))}
        </LineChart>
      </ResponsiveContainer>
    </div>
  )
}

export function SqlPanel() {
  const theme = useUi((s) => s.theme)
  const text = useUi((s) => s.sqlText)
  const setText = useUi((s) => s.setSqlText)
  const [result, setResult] = useState<QueryResponse | null>(null)
  const [showChart, setShowChart] = useState(true)

  const mutation = useMutation({
    mutationFn: () => runQuery(text),
    onSuccess: setResult,
    onError: () => setResult(null),
  })

  const run = () => mutation.mutate()

  const error = mutation.error instanceof Error ? mutation.error.message : null
  const plot = useMemo(() => (result ? detectPlot(result) : null), [result])

  return (
    <PanelGroup id="otv-sql-v" orientation="vertical">
      <Panel id="sql-editor" defaultSize={35} minSize={12}>
        <PanelChrome
          title="SQL console"
          icon={Terminal}
          className="h-full"
          bodyClassName="h-full overflow-hidden"
          actions={
            <>
              <Select
                value=""
                onValueChange={(v) => {
                  const ex = EXAMPLES.find((e) => e.label === v)
                  if (ex) setText(ex.sql)
                }}
              >
                <SelectTrigger size="sm" className="w-[170px]">
                  <SelectValue placeholder="examples" />
                </SelectTrigger>
                <SelectContent>
                  {EXAMPLES.map((e) => (
                    <SelectItem key={e.label} value={e.label}>
                      {e.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <Button size="sm" onClick={run} disabled={mutation.isPending || !text.trim()}>
                <Play />
                {mutation.isPending ? "running…" : "run"}
              </Button>
            </>
          }
        >
          <div
            className="h-full overflow-hidden text-[13px]"
            onKeyDown={(e) => {
              if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
                e.preventDefault()
                run()
              }
            }}
          >
            <CodeMirror
              value={text}
              onChange={setText}
              extensions={[sql()]}
              theme={theme === "dark" ? oneDark : "light"}
              height="100%"
              style={{ height: "100%" }}
              basicSetup={{
                foldGutter: false,
                highlightActiveLine: true,
                autocompletion: true,
              }}
            />
          </div>
        </PanelChrome>
      </Panel>
      <ResizeHandle direction="vertical" />
      <Panel id="sql-results" defaultSize={65} minSize={20}>
        <PanelChrome
          title="Results"
          icon={Table2}
          className="h-full"
          bodyClassName="flex flex-col"
          actions={
            result && (
              <>
                {plot && (
                  <Button
                    size="sm"
                    variant={showChart ? "secondary" : "ghost"}
                    onClick={() => setShowChart((v) => !v)}
                    title="Toggle chart"
                  >
                    <LineChartIcon />
                    chart
                  </Button>
                )}
                <Badge variant="secondary" className="tabular-nums">
                  {result.row_count} rows
                </Badge>
                {result.truncated && <Badge variant="destructive">truncated</Badge>}
                <span className="text-muted-foreground text-xs tabular-nums">
                  {result.elapsed_ms}ms
                </span>
              </>
            )
          }
        >
          {error ? (
            <div className="text-destructive p-3 font-mono text-xs">{error}</div>
          ) : !result ? (
            <EmptyState label="Run a query (⌘↵) to see results — read-only SELECTs against DuckDB" />
          ) : result.rows.length === 0 ? (
            <EmptyState label="Query returned no rows" />
          ) : (
            <>
              {plot && showChart && <ResultChart plot={plot} />}
              <ScrollArea className="min-h-0 flex-1" horizontal>
                <Table className="w-max min-w-full">
                  <TableHeader>
                    <TableRow>
                      {result.columns.map((c) => (
                        <TableHead key={c.name} className="pl-3 font-mono">
                          {c.name}
                          <span className="text-muted-foreground ml-1 text-[10px] font-normal">
                            {c.type}
                          </span>
                        </TableHead>
                      ))}
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {result.rows.map((row, i) => (
                      <TableRow key={i}>
                        {result.columns.map((c) => (
                          <TableCell key={c.name} className="max-w-[420px] truncate pl-3 font-mono text-xs">
                            {num(row[c.name])}
                          </TableCell>
                        ))}
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </ScrollArea>
            </>
          )}
        </PanelChrome>
      </Panel>
    </PanelGroup>
  )
}

export function SqlPreset() {
  const setSqlText = useUi((s) => s.setSqlText)
  return (
    <PanelGroup id="otv-sql-h" orientation="horizontal">
      <Panel id="sql-schema" defaultSize={22} minSize={12}>
        <SchemaPanel
          onPickTable={(t) => setSqlText(`SELECT * FROM ${t} LIMIT 100`)}
        />
      </Panel>
      <ResizeHandle direction="horizontal" />
      <Panel id="sql-main" defaultSize={78} minSize={30}>
        <SqlPanel />
      </Panel>
    </PanelGroup>
  )
}
