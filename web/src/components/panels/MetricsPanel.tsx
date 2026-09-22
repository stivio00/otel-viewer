import { useMemo, useState } from "react"
import { useQuery } from "@tanstack/react-query"
import { Bar, BarChart, CartesianGrid, Cell, Line, LineChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts"
import { Gauge, LineChart as LineChartIcon } from "lucide-react"

import { fetchMetricDetail, fetchMetrics, type MetricDetail, type MetricPointDto } from "@/lib/api"
import { fmtNsTime, num, parseJsonField, serviceColor } from "@/lib/format"
import { CHART_TOOLTIP_STYLE } from "@/lib/chart"
import { useChartAnimate } from "@/lib/hooks"
import { useDebounce } from "@/lib/hooks"
import { timeRangeStartNs, useUi } from "@/lib/store"
import { cn } from "@/lib/utils"
import { Panel, PanelGroup } from "@/components/Split"
import { EmptyState, Panel as PanelChrome } from "@/components/Panel"
import { ResizeHandle } from "@/components/Split"
import { SearchInput } from "@/components/Filters"
import { Badge } from "@/components/ui/badge"
import { ScrollArea } from "@/components/ui/scroll-area"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"

const TYPE_VARIANT: Record<string, "default" | "secondary" | "outline" | "destructive"> = {
  gauge: "secondary",
  sum: "default",
  histogram: "outline",
  "exponential_histogram": "outline",
  summary: "destructive",
}

export function MetricsPanel() {
  const [q, setQ] = useState("")
  const debouncedQ = useDebounce(q)
  const timeRange = useUi((s) => s.timeRange)
  const [selectedName, setSelectedName] = useState<string | null>(null)

  const { data, isLoading } = useQuery({
    queryKey: ["metrics", debouncedQ],
    queryFn: () => fetchMetrics(debouncedQ || undefined, 200),
    refetchInterval: 30_000,
  })

  const metrics = data?.metrics ?? []

  return (
    <PanelGroup id="otv-metrics-v" orientation="vertical">
      <Panel id="metrics-list" defaultSize={40} minSize={20}>
        <PanelChrome
          title="Metrics"
          icon={Gauge}
          className="h-full"
          bodyClassName="flex flex-col"
          actions={
            <SearchInput
              value={q}
              onChange={setQ}
              placeholder="search metrics…"
              className="relative w-[180px]"
            />
          }
        >
          <ScrollArea className="min-h-0 flex-1">
            {isLoading ? (
              <EmptyState label="Loading…" />
            ) : metrics.length === 0 ? (
              <EmptyState label="No metrics match the current filters" />
            ) : (
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="pl-3">Name</TableHead>
                    <TableHead>Type</TableHead>
                    <TableHead>Unit</TableHead>
                    <TableHead className="text-right">Services</TableHead>
                    <TableHead className="text-right">Points</TableHead>
                    <TableHead className="text-right">Last value</TableHead>
                    <TableHead className="pr-3">Last seen</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {metrics.map((m) => (
                    <TableRow
                      key={m.name}
                      onClick={() =>
                        setSelectedName(m.name === selectedName ? null : m.name)
                      }
                      className={cn("cursor-pointer", m.name === selectedName && "bg-accent/50")}
                    >
                      <TableCell className="pl-3 font-mono text-xs font-medium">{m.name}</TableCell>
                      <TableCell>
                        <Badge variant={TYPE_VARIANT[m.metric_type] ?? "secondary"}>
                          {m.metric_type}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-muted-foreground text-xs">
                        {m.unit ?? "-"}
                      </TableCell>
                      <TableCell className="text-right tabular-nums">{m.service_count}</TableCell>
                      <TableCell className="text-right tabular-nums">{m.point_count}</TableCell>
                      <TableCell className="text-right font-mono text-xs tabular-nums">
                        {m.last_value != null ? num(m.last_value) : "-"}
                      </TableCell>
                      <TableCell className="text-muted-foreground pr-3 tabular-nums">
                        {fmtNsTime(m.last_ns)}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            )}
          </ScrollArea>
        </PanelChrome>
      </Panel>
      <ResizeHandle direction="vertical" />
      <Panel id="metrics-detail" defaultSize={60} minSize={25}>
        {selectedName ? (
          <MetricDetailPanel name={selectedName} timeRangeStart={timeRangeStartNs(timeRange)} />
        ) : (
          <PanelChrome title="Metric detail" icon={LineChartIcon} className="h-full">
            <EmptyState label="Select a metric to visualize it" />
          </PanelChrome>
        )}
      </Panel>
    </PanelGroup>
  )
}

function MetricDetailPanel({
  name,
  timeRangeStart,
}: {
  name: string
  timeRangeStart: string | undefined
}) {
  const { data: detail, isLoading, error } = useQuery({
    queryKey: ["metric-detail", name, timeRangeStart],
    queryFn: () =>
      fetchMetricDetail(name, { start_ns: timeRangeStart, limit: 500 }),
    refetchInterval: 30_000,
  })

  if (isLoading) {
    return (
      <PanelChrome title="Metric detail" icon={LineChartIcon} className="h-full">
        <EmptyState label="Loading…" />
      </PanelChrome>
    )
  }
  if (error || !detail) {
    return (
      <PanelChrome title="Metric detail" icon={LineChartIcon} className="h-full">
        <EmptyState label="Metric not found" />
      </PanelChrome>
    )
  }
  return <MetricDetailBody detail={detail} />
}

interface SeriesInfo {
  key: string
  label: string
  service: string
  rows: Array<{ t: number; v: number | null }>
}

function pointValue(detail: MetricDetail, p: MetricPointDto): number | null {
  switch (detail.metric_type) {
    case "gauge":
    case "sum":
      return p.value_double ?? p.value_int ?? null
    case "histogram":
      return p.hist_count && p.hist_count > 0 ? (p.hist_sum ?? 0) / p.hist_count : null
    case "exponential_histogram": {
      const buckets = parseJsonField(p.exp_buckets)
      let count = p.exp_zero_count ?? 0
      for (const half of ["positive", "negative"]) {
        const b = buckets?.[half] as Record<string, unknown> | null | undefined
        const counts = b?.bucket_counts
        if (Array.isArray(counts)) count += counts.reduce((a: number, c) => a + Number(c), 0)
      }
      return count > 0 ? count : null
    }
    case "summary":
      return p.summary_count && p.summary_count > 0 ? (p.summary_sum ?? 0) / p.summary_count : null
    default:
      return p.value_double ?? p.value_int ?? null
  }
}


function MetricDetailBody({ detail }: { detail: MetricDetail }) {
  const animate = useChartAnimate()
  const series = useMemo<SeriesInfo[]>(() => {
    const byKey = new Map<string, SeriesInfo>()
    for (const p of detail.points) {
      const attrs = parseJsonField(p.series_attributes) ?? {}
      const attrKeys = Object.entries(attrs)
        .map(([k, v]) => `${k}=${String(v)}`)
        .join(",")
      const key = `${p.service_name}|${attrKeys}`
      let s = byKey.get(key)
      if (!s) {
        const label = attrKeys ? `${p.service_name} ${attrKeys}` : p.service_name
        s = { key, label, service: p.service_name, rows: [] }
        byKey.set(key, s)
      }
      s.rows.push({ t: Number(p.ts_ns) / 1e6, v: pointValue(detail, p) })
    }
    return [...byKey.values()]
  }, [detail])

  // Pivot rows for the line chart.
  const chartRows = useMemo(() => {
    const times = new Set<number>()
    for (const s of series) for (const r of s.rows) times.add(r.t)
    return [...times].sort((a, b) => a - b).map((t) => {
      const row: Record<string, number | null> = { t }
      for (const s of series) {
        const match = s.rows.find((r) => r.t === t)
        row[s.key] = match?.v ?? null
      }
      return row
    })
  }, [series])

  const last = detail.points[detail.points.length - 1]

  return (
    <PanelChrome
      title={<span className="font-mono normal-case">{detail.name}</span>}
      icon={LineChartIcon}
      className="h-full"
      bodyClassName="flex flex-col"
      actions={
        <>
          <Badge variant={TYPE_VARIANT[detail.metric_type] ?? "secondary"}>
            {detail.metric_type}
          </Badge>
          {detail.unit && (
            <span className="text-muted-foreground text-xs">unit: {detail.unit}</span>
          )}
          <span className="text-muted-foreground text-xs tabular-nums">
            {detail.points.length} points
          </span>
        </>
      }
    >
      <div className="flex min-h-0 flex-1 flex-col gap-2 p-3">
        {detail.description && (
          <p className="text-muted-foreground text-xs">{detail.description}</p>
        )}

        {chartRows.length === 0 ? (
          <EmptyState label="No points in range" />
        ) : (
          <div className="min-h-[180px] flex-1">
            <ResponsiveContainer width="100%" height="100%">
              <LineChart data={chartRows} margin={{ top: 4, right: 8, bottom: 0, left: 0 }}>
                <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" />
                <XAxis
                  dataKey="t"
                  type="number"
                  scale="time"
                  domain={["dataMin", "dataMax"]}
                  tickFormatter={(t) => fmtNsTime(t * 1e6)}
                  stroke="var(--muted-foreground)"
                  fontSize={10}
                  tickMargin={4}
                />
                <YAxis stroke="var(--muted-foreground)" fontSize={10} width={44} />
                <Tooltip
                  contentStyle={CHART_TOOLTIP_STYLE}
                  labelFormatter={(t) => fmtNsTime(Number(t) * 1e6)}
                />
                {series.map((s) => (
                  <Line
                    key={s.key}
                    dataKey={s.key}
                    name={s.label}
                    type="monotone"
                    dot={false}
                    strokeWidth={1.8}
                    connectNulls
                    stroke={serviceColor(s.service)}
                    isAnimationActive={animate}
                  />
                ))}
              </LineChart>
            </ResponsiveContainer>
          </div>
        )}

        {detail.metric_type === "histogram" && last && (
          <HistogramBuckets bounds={last.hist_bounds} counts={last.hist_bucket_counts} />
        )}
        {detail.metric_type === "exponential_histogram" && last && (
          <ExpBuckets buckets={last.exp_buckets} zeroCount={last.exp_zero_count} />
        )}
        {detail.metric_type === "summary" && last && <SummaryQuantiles data={last.summary_quantiles} />}
      </div>
    </PanelChrome>
  )
}

const BUCKET_COLORS = ["var(--chart-1)", "var(--chart-2)", "var(--chart-3)", "var(--chart-4)", "var(--chart-5)"]

function HistogramBuckets({ bounds, counts }: { bounds: unknown; counts: unknown }) {
  const animate = useChartAnimate()
  const bArr = Array.isArray(bounds) ? bounds.map(Number) : []
  const cArr = Array.isArray(counts) ? counts.map(Number) : []
  if (cArr.length === 0) return null
  const data = cArr.map((c, i) => ({
    bucket: i < bArr.length ? `≤${bArr[i]}` : `>${bArr[bArr.length - 1] ?? "∞"}`,
    count: c,
  }))
  return (
    <div>
      <p className="text-muted-foreground mb-1 text-[10px] tracking-wide uppercase">
        last point buckets
      </p>
      <div className="h-24">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart data={data}>
            <XAxis dataKey="bucket" stroke="var(--muted-foreground)" fontSize={9} />
            <YAxis stroke="var(--muted-foreground)" fontSize={9} width={32} />
            <Tooltip
              contentStyle={CHART_TOOLTIP_STYLE}
            />
            <Bar dataKey="count" radius={[3, 3, 0, 0]} isAnimationActive={animate}>
              {data.map((_, i) => (
                <Cell key={i} fill={BUCKET_COLORS[i % BUCKET_COLORS.length]} />
              ))}
            </Bar>
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  )
}

function ExpBuckets({ buckets, zeroCount }: { buckets: unknown; zeroCount: number | null }) {
  const animate = useChartAnimate()
  const parsed = parseJsonField(buckets)
  const positive = parsed?.positive as Record<string, unknown> | undefined
  const offset = Number(positive?.offset ?? 0)
  const counts = Array.isArray(positive?.bucket_counts)
    ? (positive!.bucket_counts as unknown[]).map(Number)
    : []
  if (counts.length === 0 && !zeroCount) return null
  const data = [
    { bucket: "zero", count: zeroCount ?? 0 },
    ...counts.map((c, i) => ({ bucket: `2^${offset + i - 1}`, count: c })),
  ]
  return (
    <div>
      <p className="text-muted-foreground mb-1 text-[10px] tracking-wide uppercase">
        last point buckets
      </p>
      <div className="h-24">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart data={data}>
            <XAxis dataKey="bucket" stroke="var(--muted-foreground)" fontSize={9} />
            <YAxis stroke="var(--muted-foreground)" fontSize={9} width={32} />
            <Tooltip
              contentStyle={CHART_TOOLTIP_STYLE}
            />
            <Bar dataKey="count" radius={[3, 3, 0, 0]} fill="var(--chart-1)" isAnimationActive={animate} />
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  )
}

/** Quantiles come either as OTLP objects `{quantile, value}` or `[q, v]` pairs. */
function parseQuantiles(data: unknown): Array<{ q: number; v: number }> {
  const parsed = parseJsonField(data)
  if (!Array.isArray(parsed)) return []
  const out: Array<{ q: number; v: number }> = []
  for (const entry of parsed) {
    let q: number
    let v: number
    if (Array.isArray(entry) && entry.length >= 2) {
      q = Number(entry[0])
      v = Number(entry[1])
    } else if (entry != null && typeof entry === "object") {
      const o = entry as Record<string, unknown>
      q = Number(o.quantile)
      v = Number(o.value)
    } else {
      continue
    }
    if (Number.isFinite(q) && Number.isFinite(v)) out.push({ q, v })
  }
  return out.sort((a, b) => a.q - b.q)
}

function SummaryQuantiles({ data }: { data: unknown }) {
  const quantiles = parseQuantiles(data)
  if (quantiles.length === 0) return null
  return (
    <div>
      <p className="text-muted-foreground mb-1 text-[10px] tracking-wide uppercase">
        last point quantiles
      </p>
      <div className="overflow-hidden rounded-md border">
        {quantiles.map(({ q, v }, i) => (
          <div
            key={i}
            className={cn(
              "flex items-center justify-between px-2 py-1 font-mono text-[11px]",
              i % 2 === 1 && "bg-muted/30"
            )}
          >
            <span className="text-muted-foreground">p{(q * 100).toFixed(0)}</span>
            <span>{num(v)}</span>
          </div>
        ))}
      </div>
    </div>
  )
}
