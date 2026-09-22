import { useMemo, useState } from "react"
import { keepPreviousData, useQuery } from "@tanstack/react-query"
import {
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Line,
  LineChart,
  PolarAngleAxis,
  RadialBar,
  RadialBarChart,
  ResponsiveContainer,
  Scatter,
  ScatterChart,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts"
import { LayoutDashboard } from "lucide-react"

import {
  fetchDashboard,
  fetchDashboards,
  fetchServices,
  runQuery,
  type QueryResponse,
} from "@/lib/api"
import {
  CHART_TOOLTIP_STYLE,
  detectBar,
  detectHeat,
  detectHist,
  detectPlot,
  detectScatter,
  toNum,
  type PlotShape,
} from "@/lib/chart"
import { useChartAnimate } from "@/lib/hooks"
import {
  attributeOptionsSql,
  renderSql,
  type DashboardInput,
  type DashboardPanel,
  type PanelType,
} from "@/lib/dash"
import { fmtNsTime, num, serviceColor } from "@/lib/format"
import { cn } from "@/lib/utils"
import { Panel, PanelGroup, ResizeHandle } from "@/components/Split"
import { EmptyState, Panel as PanelChrome } from "@/components/Panel"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"

const ALL = "__all__"

const TIME_PRESETS: Array<{ value: string; label: string; ms: number | null }> = [
  { value: "all", label: "All", ms: null },
  { value: "5m", label: "5m", ms: 5 * 60_000 },
  { value: "10m", label: "10m", ms: 10 * 60_000 },
  { value: "15m", label: "15m", ms: 15 * 60_000 },
  { value: "30m", label: "30m", ms: 30 * 60_000 },
  { value: "1h", label: "1h", ms: 3_600_000 },
  { value: "3h", label: "3h", ms: 3 * 3_600_000 },
  { value: "6h", label: "6h", ms: 6 * 3_600_000 },
  { value: "12h", label: "12h", ms: 12 * 3_600_000 },
  { value: "24h", label: "24h", ms: 24 * 3_600_000 },
  { value: "7d", label: "7d", ms: 7 * 24 * 3_600_000 },
]

/** Quick window nudges shown next to the from/to pickers. */
const QUICK_SHIFTS: Array<{ label: string; ms: number }> = [
  { label: "+10m", ms: 10 * 60_000 },
  { label: "+15m", ms: 15 * 60_000 },
  { label: "+1h", ms: 3_600_000 },
]

function windowLabel(fromMs: number, toMs: number): string {
  const w = Math.max(0, toMs - fromMs)
  if (w >= 86_400_000) return `${round(w / 86_400_000, 1)}d`
  if (w >= 3_600_000) return `${round(w / 3_600_000, 1)}h`
  if (w >= 60_000) return `${round(w / 60_000, 1)}m`
  return `${round(w / 1000, 0)}s`
}

function round(n: number, digits: number): number {
  const p = 10 ** digits
  return Math.round(n * p) / p
}

const MAX_TO_NS = 9.2e18 // int64-ish upper bound = "now and beyond"

export function DashboardsPanel() {
  const { data: list, isLoading } = useQuery({
    queryKey: ["dashboards"],
    queryFn: fetchDashboards,
  })
  const [selected, setSelected] = useState<string | null>(null)
  const active = selected ?? list?.dashboards[0]?.id ?? null

  return (
    <PanelGroup id="otv-dash-h" orientation="horizontal">
      <Panel id="dash-list" defaultSize={22} minSize={14}>
        <PanelChrome
          title="Dashboards"
          icon={LayoutDashboard}
          className="h-full"
          bodyClassName="p-1.5"
        >
          {isLoading && <EmptyState label="Loading…" />}
          {list && list.dashboards.length === 0 && (
            <EmptyState label="No dashboards found — drop a YAML into ~/.otel-viewer/dashboards/" />
          )}
          <div className="space-y-1">
            {list?.dashboards.map((d) => (
              <button
                key={d.id}
                onClick={() => setSelected(d.id)}
                className={cn(
                  "block w-full rounded-md px-2 py-1.5 text-left",
                  d.id === active ? "bg-accent" : "hover:bg-accent/50"
                )}
              >
                <div className="flex items-center gap-1.5 text-xs font-medium">
                  <span className="truncate">{d.title}</span>
                  {d.source === "user" && <Badge variant="outline">user</Badge>}
                </div>
                <div className="text-muted-foreground mt-0.5 truncate text-[10px]">
                  {d.description.split("\n")[0]}
                </div>
              </button>
            ))}
          </div>
        </PanelChrome>
      </Panel>
      <ResizeHandle direction="horizontal" />
      <Panel id="dash-view" defaultSize={78} minSize={30}>
        {active ? (
          <DashboardView key={active} id={active} />
        ) : (
          <PanelChrome title="Dashboard" icon={LayoutDashboard} className="h-full">
            <EmptyState label="Select a dashboard" />
          </PanelChrome>
        )}
      </Panel>
    </PanelGroup>
  )
}

interface RangeState {
  preset: string
  fromMs: number | null
  toMs: number | null
}

function DashboardView({ id }: { id: string }) {
  const { data: dash, isLoading, error } = useQuery({
    queryKey: ["dashboard", id],
    queryFn: () => fetchDashboard(id),
  })
  const [range, setRange] = useState<RangeState>({ preset: "all", fromMs: null, toMs: null })
  const [inputs, setInputs] = useState<Record<string, string>>({})

  const effectiveInputs = useMemo(() => {
    const out: Record<string, string> = {}
    for (const i of dash?.spec.inputs ?? []) out[i.name] = inputs[i.name] ?? i.default ?? ""
    return out
  }, [dash, inputs])

  const bounds = useMemo(() => {
    if (range.preset === "all") return { fromNs: 0, toNs: MAX_TO_NS }
    if (range.preset === "custom" && range.fromMs != null && range.toMs != null)
      return { fromNs: range.fromMs * 1e6, toNs: range.toMs * 1e6 }
    const win = TIME_PRESETS.find((p) => p.value === range.preset)?.ms ?? 3_600_000
    const to = range.toMs ?? Date.now()
    return { fromNs: (to - win) * 1e6, toNs: to * 1e6 }
  }, [range])

  const windowWidth = () => {
    if (range.preset === "custom" && range.fromMs != null && range.toMs != null)
      return Math.max(1_000, range.toMs - range.fromMs)
    return TIME_PRESETS.find((p) => p.value === range.preset)?.ms ?? 3_600_000
  }

  const shiftRange = (ms: number) => {
    setRange({
      preset: "custom",
      fromMs: Math.max(0, bounds.fromNs / 1e6 + ms),
      toMs: bounds.toNs / 1e6 + ms,
    })
  }

  const jumpToNow = () => {
    const w = windowWidth()
    const now = Date.now()
    setRange({ preset: "custom", fromMs: now - w, toMs: now })
  }

  if (isLoading) {
    return (
      <PanelChrome title="Dashboard" icon={LayoutDashboard} className="h-full">
        <EmptyState label="Loading…" />
      </PanelChrome>
    )
  }
  if (error || !dash) {
    return (
      <PanelChrome title="Dashboard" icon={LayoutDashboard} className="h-full">
        <EmptyState label="Dashboard not found" />
      </PanelChrome>
    )
  }

  return (
    <PanelChrome
      title={dash.title}
      icon={LayoutDashboard}
      className="h-full"
      bodyClassName="flex flex-col"
      actions={<Badge variant="outline">{dash.source}</Badge>}
    >
      <div className="text-muted-foreground max-h-20 shrink-0 overflow-auto whitespace-pre-line px-3 py-1.5 text-[11px]">
        {dash.description}
      </div>

      <div className="bg-muted/30 flex flex-wrap items-center gap-x-3 gap-y-2 border-b px-3 py-2">
        <div className="flex items-center gap-0.5">
          {TIME_PRESETS.map((p) => (
            <Button
              key={p.value}
              size="sm"
              variant={range.preset === p.value ? "default" : "ghost"}
              className="h-6 px-1.5 text-[11px]"
              onClick={() => setRange({ preset: p.value, fromMs: null, toMs: null })}
            >
              {p.label}
            </Button>
          ))}
        </div>

        {range.preset !== "all" && (
          <div className="flex items-center gap-0.5">
            <Button
              size="sm"
              variant="outline"
              className="h-6 px-1.5 text-[11px]"
              title="Shift window back by its width"
              onClick={() => shiftRange(-windowWidth())}
            >
              ‹
            </Button>
            <Button
              size="sm"
              variant="outline"
              className="h-6 px-1.5 text-[11px]"
              title="Shift window forward by its width"
              onClick={() => shiftRange(windowWidth())}
            >
              ›
            </Button>
            <Button
              size="sm"
              variant="ghost"
              className="h-6 px-2 text-[11px]"
              title="Jump to the latest data, keep the window width"
              onClick={jumpToNow}
            >
              Now
            </Button>
            <span className="bg-background text-muted-foreground rounded border px-1.5 py-0.5 text-[10px] tabular-nums">
              {windowLabel(bounds.fromNs / 1e6, bounds.toNs / 1e6)}
            </span>
          </div>
        )}

        <div className="flex items-center gap-1.5 text-[11px]">
          <label className="flex items-center gap-1">
            <span className="text-muted-foreground">From</span>
            <input
              type="datetime-local"
              step={1}
              className="bg-background border-input rounded-md border px-1.5 py-1 text-[11px]"
              value={toLocalInput(
                range.preset === "custom"
                  ? (range.fromMs ?? 0)
                  : bounds.fromNs > 0
                    ? bounds.fromNs / 1e6
                    : Date.now() - 3_600_000
              )}
              onChange={(e) =>
                setRange((r) => ({
                  preset: "custom",
                  fromMs: e.target.value ? Date.parse(e.target.value) : null,
                  toMs: r.preset === "custom" ? r.toMs : null,
                }))
              }
            />
          </label>
          <span className="text-muted-foreground">→</span>
          <label className="flex items-center gap-1">
            <span className="text-muted-foreground">To</span>
            <input
              type="datetime-local"
              step={1}
              className="bg-background border-input rounded-md border px-1.5 py-1 text-[11px]"
              value={toLocalInput(
                range.preset === "custom"
                  ? (range.toMs ?? Date.now())
                  : bounds.toNs < MAX_TO_NS
                    ? bounds.toNs / 1e6
                    : Date.now()
              )}
              onChange={(e) =>
                setRange((r) => ({
                  preset: "custom",
                  fromMs: r.preset === "custom" ? r.fromMs : Date.now() - 3_600_000,
                  toMs: e.target.value ? Date.parse(e.target.value) : null,
                }))
              }
            />
          </label>
          {range.preset !== "all" && (
            <div className="flex items-center gap-0.5">
              {QUICK_SHIFTS.map((s) => (
                <Button
                  key={s.label}
                  size="sm"
                  variant="ghost"
                  className="text-muted-foreground h-6 px-1.5 text-[10px]"
                  title={`Move window ${s.label}`}
                  onClick={() => shiftRange(s.ms)}
                >
                  {s.label}
                </Button>
              ))}
            </div>
          )}
        </div>

        <div className="ml-auto flex flex-wrap items-center gap-2">
          {(dash.spec.inputs ?? []).map((i) => (
            <InputSelect
              key={i.name}
              dashId={id}
              input={i}
              value={effectiveInputs[i.name] ?? ""}
              onChange={(v) => setInputs((prev) => ({ ...prev, [i.name]: v }))}
            />
          ))}
        </div>
      </div>

      <div className="grid min-h-0 flex-1 auto-rows-min grid-cols-1 gap-2 overflow-auto p-2 xl:grid-cols-2">
        {(dash.spec.panels ?? []).map((p) => (
          <DashPanel
            key={p.id}
            dashId={id}
            panel={p}
            fromNs={bounds.fromNs}
            toNs={bounds.toNs}
            inputs={effectiveInputs}
          />
        ))}
      </div>
    </PanelChrome>
  )
}

function InputSelect({
  dashId,
  input,
  value,
  onChange,
}: {
  dashId: string
  input: DashboardInput
  value: string
  onChange: (v: string) => void
}) {
  const services = useQuery({
    queryKey: ["services"],
    queryFn: fetchServices,
    enabled: input.type === "service",
  })
  const attr = useQuery({
    queryKey: ["dash-attr", dashId, input.name],
    queryFn: () => runQuery(attributeOptionsSql(input) ?? "SELECT 1 WHERE false", 500),
    enabled: input.type === "attribute",
  })

  let options: Array<{ value: string; label: string }> = []
  if (input.type === "service") {
    options = (services.data?.services ?? []).map((s) => ({ value: s.name, label: s.name }))
  } else if (input.type === "attribute") {
    options = (attr.data?.rows ?? [])
      .map((r) => String((r as Record<string, unknown>).v ?? ""))
      .filter((v) => v !== "" && v !== "null")
      .sort()
      .map((v) => ({ value: v, label: v }))
  } else {
    options = (input.choices ?? []).map((c) => ({ value: c.value, label: c.label ?? c.value }))
  }

  return (
    <label className="flex items-center gap-1.5 text-[11px]">
      <span className="text-muted-foreground">{input.label ?? input.name}</span>
      <Select value={value === "" ? ALL : value} onValueChange={(v) => onChange(v === ALL ? "" : v)}>
        <SelectTrigger size="sm" className="h-6 w-[130px] text-[11px]">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value={ALL}>(any)</SelectItem>
          {options.map((o) => (
            <SelectItem key={o.value} value={o.value}>
              {o.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </label>
  )
}

function DashPanel({
  dashId,
  panel,
  fromNs,
  toNs,
  inputs,
}: {
  dashId: string
  panel: DashboardPanel
  fromNs: number
  toNs: number
  inputs: Record<string, string>
}) {
  const sql = useMemo(
    () => renderSql(panel.sql ?? "", { fromNs, toNs, inputs }),
    [panel.sql, fromNs, toNs, inputs]
  )
  // keepPreviousData: when the time range / an input changes, the previous
  // data stays visible until the new result arrives — no loading flash.
  const { data, isLoading, error } = useQuery({
    queryKey: ["dash-panel", dashId, panel.id, sql],
    queryFn: () => runQuery(sql, 5000),
    enabled: !!panel.sql,
    placeholderData: keepPreviousData,
  })

  return (
    <PanelChrome
      title={panel.title ?? panel.id}
      className={cn("h-[300px]", (panel.span ?? 1) >= 2 && "xl:col-span-2")}
      bodyClassName="p-2"
      actions={panel.unit ? <Badge variant="secondary">{panel.unit}</Badge> : null}
    >
      {isLoading && <EmptyState label="Loading…" />}
      {error && (
        <div className="text-destructive flex h-full items-center justify-center px-4 text-center text-[11px]">
          {(error as Error).message}
        </div>
      )}
      {data && <PanelBody type={panel.type} result={data} max={panel.max} />}
    </PanelChrome>
  )
}

function PanelBody({
  type,
  result,
  max,
}: {
  type: PanelType
  result: QueryResponse
  max?: number
}) {
  if (result.rows.length === 0) return <EmptyState label="No data in range" />

  switch (type) {
    case "line": {
      const plot = detectPlot(result)
      return plot ? <LineView plot={plot} /> : <EmptyState label="No plottable columns" />
    }
    case "points": {
      const s = detectScatter(result)
      return s ? <PointsView shape={s} /> : <EmptyState label="No plottable columns" />
    }
    case "bar": {
      const b = detectBar(result)
      return b ? <BarView shape={b} /> : <EmptyState label="No plottable columns" />
    }
    case "histogram": {
      const h = detectHist(result)
      return h ? <HistView shape={h} /> : <EmptyState label="No numeric column" />
    }
    case "dial": {
      const v = lastNumeric(result)
      return v == null ? <EmptyState label="No value" /> : <DialView value={v} max={max} />
    }
    case "stat": {
      const v = lastNumeric(result)
      return v == null ? <EmptyState label="No value" /> : <StatView value={v} />
    }
    case "heatmap": {
      const h = detectHeat(result)
      return h ? <HeatView shape={h} /> : <EmptyState label="No x/y/weight columns" />
    }
    case "table":
      return <TableView result={result} />
    default:
      return <EmptyState label={`Unknown panel type: ${type}`} />
  }
}

/** Latest (last row, scanning back) value of the first numeric column. */
function lastNumeric(result: QueryResponse): number | null {
  const col = result.columns.find((c) => result.rows.some((r) => toNum(r[c.name]) != null))
  if (!col) return null
  for (let i = result.rows.length - 1; i >= 0; i--) {
    const v = toNum(result.rows[i][col.name])
    if (v != null) return v
  }
  return null
}

function LineView({ plot }: { plot: PlotShape }) {
  const animate = useChartAnimate()
  return (
    <div className="h-full w-full">
      <ResponsiveContainer>
        <LineChart data={plot.rows} margin={{ top: 6, right: 8, bottom: 0, left: 0 }}>
          <CartesianGrid stroke="var(--border)" strokeDasharray="3 3" />
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
            contentStyle={CHART_TOOLTIP_STYLE}
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
              isAnimationActive={animate}
            />
          ))}
        </LineChart>
      </ResponsiveContainer>
    </div>
  )
}

function PointsView({
  shape,
}: {
  shape: { xCol: string; yCols: string[]; rows: Array<Record<string, number | null>> }
}) {
  const animate = useChartAnimate()
  return (
    <div className="h-full w-full">
      <ResponsiveContainer>
        <ScatterChart margin={{ top: 6, right: 8, bottom: 0, left: 0 }}>
          <CartesianGrid stroke="var(--border)" strokeDasharray="3 3" />
          <XAxis
            dataKey="x"
            type="number"
            stroke="var(--muted-foreground)"
            fontSize={10}
            tickFormatter={(t) =>
              Math.abs(Number(t)) > 1e11 ? fmtNsTime(Number(t) * 1e6) : num(t)
            }
          />
          <YAxis stroke="var(--muted-foreground)" fontSize={10} width={48} />
          <Tooltip contentStyle={CHART_TOOLTIP_STYLE} />
          {shape.yCols.map((name) => (
            <Scatter
              key={name}
              name={name}
              data={shape.rows}
              dataKey={name}
              fill={serviceColor(name)}
              fillOpacity={0.7}
              isAnimationActive={animate}
            />
          ))}
        </ScatterChart>
      </ResponsiveContainer>
    </div>
  )
}

function BarView({
  shape,
}: {
  shape: { labelCol: string; valueCols: string[]; rows: Array<Record<string, string | number | null>> }
}) {
  const animate = useChartAnimate()
  return (
    <div className="h-full w-full">
      <ResponsiveContainer>
        <BarChart data={shape.rows} margin={{ top: 6, right: 8, bottom: 0, left: 0 }}>
          <CartesianGrid stroke="var(--border)" strokeDasharray="3 3" vertical={false} />
          <XAxis
            dataKey="label"
            stroke="var(--muted-foreground)"
            fontSize={10}
            tickFormatter={(l: string) => (l.length > 14 ? `${l.slice(0, 13)}…` : l)}
            interval={0}
            angle={-15}
            textAnchor="end"
            height={44}
          />
          <YAxis stroke="var(--muted-foreground)" fontSize={10} width={48} />
          <Tooltip contentStyle={CHART_TOOLTIP_STYLE} />
          {shape.valueCols.map((name) => (
            <Bar
              key={name}
              dataKey={name}
              name={name}
              fill={serviceColor(name)}
              radius={[3, 3, 0, 0]}
              isAnimationActive={animate}
            />
          ))}
        </BarChart>
      </ResponsiveContainer>
    </div>
  )
}

function HistView({
  shape,
}: {
  shape: {
    col: string
    bins: Array<{ x0: number; x1: number; count: number }>
    min: number
    max: number
  }
}) {
  const animate = useChartAnimate()
  const data = shape.bins.map((b) => ({ ...b, label: `${num(b.x0)}–${num(b.x1)}` }))
  return (
    <div className="flex h-full w-full flex-col">
      <div className="text-muted-foreground px-1 pb-1 text-[10px]">
        {shape.col} · {num(shape.min)} – {num(shape.max)}
      </div>
      <div className="min-h-0 flex-1">
        <ResponsiveContainer>
          <BarChart data={data} margin={{ top: 4, right: 8, bottom: 0, left: 0 }}>
            <CartesianGrid stroke="var(--border)" strokeDasharray="3 3" vertical={false} />
            <XAxis
              dataKey="label"
              stroke="var(--muted-foreground)"
              fontSize={9}
              interval={0}
              angle={-30}
              textAnchor="end"
              height={40}
            />
            <YAxis stroke="var(--muted-foreground)" fontSize={10} width={36} />
            <Tooltip contentStyle={CHART_TOOLTIP_STYLE} />
            <Bar dataKey="count" name="count" radius={[3, 3, 0, 0]} isAnimationActive={animate}>
              {data.map((_, i) => (
                <Cell key={i} fill={serviceColor(String(i))} />
              ))}
            </Bar>
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  )
}

function DialView({ value, max }: { value: number; max?: number }) {
  const animate = useChartAnimate()
  // Nice rounded auto scale: ceil(|value|) to a 1/2/5 × 10^k step.
  const m = Math.max(Math.abs(value), 1e-9)
  const step = Math.pow(10, Math.floor(Math.log10(m))) / 2
  const auto = Math.ceil(m / step) * step
  const displayMax = max ?? auto
  return (
    <div className="relative h-full w-full">
      <ResponsiveContainer>
        <RadialBarChart
          data={[{ name: "v", value }]}
          innerRadius="70%"
          outerRadius="100%"
          startAngle={210}
          endAngle={-30}
        >
          <PolarAngleAxis type="number" domain={[0, displayMax]} tick={false} />
          <RadialBar
            dataKey="value"
            isAnimationActive={animate}
            cornerRadius={8}
            fill="var(--chart-1, #3b82f6)"
            background={{ fill: "var(--muted)" }}
          />
        </RadialBarChart>
      </ResponsiveContainer>
      <div className="pointer-events-none absolute inset-0 flex flex-col items-center justify-center pb-4">
        <span className="text-2xl font-semibold tabular-nums">{num(value)}</span>
        <span className="text-muted-foreground text-[10px]">
          max {num(displayMax)}
        </span>
      </div>
    </div>
  )
}

function StatView({ value }: { value: number }) {
  return (
    <div className="flex h-full w-full items-center justify-center">
      <span className="text-4xl font-semibold tabular-nums">{num(value)}</span>
    </div>
  )
}

function HeatView({
  shape,
}: {
  shape: {
    cells: Array<{ x: number; y: number; weight: number }>
    xs: number[]
    ys: number[]
    maxWeight: number
  }
}) {
  const byKey = new Map(shape.cells.map((c) => [`${c.x}|${c.y}`, c.weight]))
  return (
    <div className="flex h-full w-full gap-1 overflow-hidden">
      <div
        className="text-muted-foreground grid shrink-0 items-center text-right text-[9px] tabular-nums"
        style={{ gridTemplateRows: `repeat(${shape.ys.length}, 1fr)` }}
      >
        {shape.ys.map((y) => (
          <span key={y} className="truncate">{num(y)}</span>
        ))}
      </div>
      <div className="min-w-0 flex-1">
        <div
          className="grid h-[calc(100%-14px)] gap-px"
          style={{
            gridTemplateColumns: `repeat(${shape.xs.length}, 1fr)`,
            gridTemplateRows: `repeat(${shape.ys.length}, 1fr)`,
          }}
        >
          {shape.xs.flatMap((x) =>
            shape.ys.map((y) => {
              const w = byKey.get(`${x}|${y}`)
              const a = w == null || w <= 0 ? 0 : 0.08 + 0.92 * Math.pow(w / shape.maxWeight, 0.6)
              return (
                <div
                  key={`${x}|${y}`}
                  className="rounded-[1px]"
                  style={{ backgroundColor: `rgba(59,130,246,${a.toFixed(3)})` }}
                  title={`${fmtNsTime(x * 1e6)} · ${num(y)}s · ${w ?? 0}`}
                />
              )
            })
          )}
        </div>
        <div className="text-muted-foreground flex justify-between pt-0.5 text-[9px] tabular-nums">
          <span>{shape.xs.length ? fmtNsTime(shape.xs[0] * 1e6) : ""}</span>
          <span>
            {shape.xs.length ? fmtNsTime(shape.xs[Math.floor(shape.xs.length / 2)] * 1e6) : ""}
          </span>
          <span>{shape.xs.length ? fmtNsTime(shape.xs[shape.xs.length - 1] * 1e6) : ""}</span>
        </div>
      </div>
    </div>
  )
}

function TableView({ result }: { result: QueryResponse }) {
  const numeric = new Set(
    result.columns.filter((c) => result.rows.some((r) => toNum(r[c.name]) != null)).map((c) => c.name)
  )
  return (
    <div className="h-full overflow-auto">
      <table className="w-full border-collapse text-xs">
        <thead className="bg-muted/60 sticky top-0 z-10">
          <tr>
            {result.columns.map((c) => (
              <th
                key={c.name}
                className={cn(
                  "text-muted-foreground px-2 py-1 text-left font-medium whitespace-nowrap",
                  numeric.has(c.name) && "text-right"
                )}
              >
                {c.name}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {result.rows.map((r, i) => (
            <tr key={i} className={cn("border-t", i % 2 === 1 && "bg-muted/20")}>
              {result.columns.map((c, ci) => {
                const v = r[c.name]
                return (
                  <td
                    key={c.name}
                    title={String(v ?? "")}
                    className={cn(
                      "px-2 py-1 whitespace-nowrap",
                      ci === 0 && "max-w-56 truncate font-medium",
                      numeric.has(c.name) && "text-right tabular-nums",
                      String(r[result.columns[0].name] ?? "") === "Total" && "border-t-2 font-semibold"
                    )}
                  >
                    {v == null
                      ? "—"
                      : typeof v === "number"
                        ? num(v)
                        : numeric.has(c.name)
                          ? num(toNum(v) ?? 0)
                          : String(v)}
                  </td>
                )
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

function toLocalInput(ms: number): string {
  const d = new Date(ms)
  const p = (n: number) => String(n).padStart(2, "0")
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`
}
