import { useEffect, useRef, useState } from "react"
import { useQuery, useQueryClient } from "@tanstack/react-query"
import { Activity, Info, Moon, RefreshCw, Sun, Timer, Trash2 } from "lucide-react"

import { fetchDbStats, fetchHealth, fetchStats, resetDb } from "@/lib/api"
import { AUTO_REFRESH_OPTIONS, useUi, type Preset, type TimeRange } from "@/lib/store"
import { Button } from "@/components/ui/button"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"

const PRESETS: Array<{ value: Preset; label: string }> = [
  { value: "default", label: "Default" },
  { value: "traces", label: "Traces" },
  { value: "logs", label: "Logs" },
  { value: "metrics", label: "Metrics" },
  { value: "sql", label: "SQL" },
  { value: "dashboards", label: "Dashboards" },
]

const RANGES: Array<{ value: TimeRange; label: string }> = [
  { value: "5m", label: "Last 5m" },
  { value: "15m", label: "Last 15m" },
  { value: "1h", label: "Last 1h" },
  { value: "6h", label: "Last 6h" },
  { value: "24h", label: "Last 24h" },
  { value: "all", label: "All time" },
]

function StatChip({ label, value }: { label: string; value: string }) {
  return (
    <div className="bg-muted/60 hidden items-center gap-1.5 rounded-md px-2 py-1 md:flex">
      <span className="text-muted-foreground text-[10px] tracking-wide uppercase">{label}</span>
      <span className="text-xs font-semibold tabular-nums">{value}</span>
    </div>
  )
}

function fmtBytes(n: number | null | undefined): string {
  if (n == null) return "—"
  if (n < 1024) return `${n} B`
  if (n < 1024 ** 2) return `${(n / 1024).toFixed(1)} KiB`
  if (n < 1024 ** 3) return `${(n / 1024 ** 2).toFixed(1)} MiB`
  return `${(n / 1024 ** 3).toFixed(2)} GiB`
}

function fmtNum(n: number | null | undefined): string {
  return n == null ? "—" : n.toLocaleString()
}

function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-baseline justify-between gap-3 py-0.5">
      <span className="text-muted-foreground text-[10px] tracking-wide uppercase">{label}</span>
      <span className="text-xs font-semibold tabular-nums">{value}</span>
    </div>
  )
}

/** DuckDB storage info, toggled from the header (i) button. */
function DbInfoPopover({ onClose }: { onClose: () => void }) {
  const { data: db } = useQuery({
    queryKey: ["dbstats"],
    queryFn: fetchDbStats,
    refetchInterval: 5_000,
  })
  return (
    <>
      <div className="fixed inset-0 z-40" onClick={onClose} />
      <div className="bg-popover text-popover-foreground absolute top-11 right-2 z-50 w-90 max-w-[calc(100vw-16px)] rounded-md border p-3 shadow-lg">
        <div className="mb-1 flex items-center justify-between">
          <span className="text-xs font-bold">DuckDB storage</span>
          <span className="text-muted-foreground text-[10px]">live</span>
        </div>
        <div className="mb-2 border-b pb-2">
          <InfoRow
            label="file"
            value={db?.db_file ? db.db_file.split("/").pop() ?? db.db_file : "in-memory"}
          />
          {db?.db_file && (
            <div className="text-muted-foreground font-mono text-[10px] break-all">{db.db_file}</div>
          )}
        </div>
        <div className="border-b pb-2">
          <InfoRow label="file size" value={fmtBytes(db?.file_size_bytes)} />
          <InfoRow label="db size" value={db?.database_size ?? "—"} />
          <InfoRow
            label="blocks"
            value={`${fmtNum(db?.used_blocks)} used / ${fmtNum(db?.total_blocks)} (${fmtBytes(
              db?.block_size
            )} each)`}
          />
          <InfoRow label="free blocks" value={fmtNum(db?.free_blocks)} />
          <InfoRow label="checkpoints" value={fmtNum(db?.checkpoint_count)} />
          <InfoRow label="memory" value={fmtBytes(db?.memory_bytes)} />
        </div>
        <div className="mt-2 space-y-1">
          <div className="text-muted-foreground text-[10px] tracking-wide uppercase">tables</div>
          {db?.tables.map((t) => (
            <div key={t.table_name} className="flex items-baseline justify-between gap-3">
              <span className="font-mono text-xs">{t.table_name}</span>
              <span className="text-muted-foreground text-[11px] tabular-nums">
                {fmtNum(t.estimated_size)} rows · {fmtNum(t.column_count)} cols ·{" "}
                {fmtNum(t.index_count)} idx
              </span>
            </div>
          ))}
        </div>
      </div>
    </>
  )
}

export function AppHeader() {
  const { data: stats } = useQuery({
    queryKey: ["stats"],
    queryFn: fetchStats,
    refetchInterval: 5_000,
  })
  const { data: health } = useQuery({
    queryKey: ["health"],
    queryFn: fetchHealth,
    staleTime: 60_000,
  })
  const theme = useUi((s) => s.theme)
  const toggleTheme = useUi((s) => s.toggleTheme)
  const preset = useUi((s) => s.preset)
  const setPreset = useUi((s) => s.setPreset)
  const timeRange = useUi((s) => s.timeRange)
  const setTimeRange = useUi((s) => s.setTimeRange)
  const autoRefreshMs = useUi((s) => s.autoRefreshMs)
  const setAutoRefreshMs = useUi((s) => s.setAutoRefreshMs)
  const queryClient = useQueryClient()

  const [confirmReset, setConfirmReset] = useState(false)
  const [resetError, setResetError] = useState<string | null>(null)
  const resetTimer = useRef<number>(0)
  const [infoOpen, setInfoOpen] = useState(false)

  // Auto-refresh: invalidate active queries on an interval. Invalidation
  // refetches in place — previous data stays on screen (no loading flash,
  // no chart remount), unlike changing query keys. 0 = off.
  useEffect(() => {
    if (!autoRefreshMs) return
    const id = window.setInterval(() => queryClient.invalidateQueries(), autoRefreshMs)
    return () => window.clearInterval(id)
  }, [autoRefreshMs, queryClient])

  // window.confirm() is a silent no-op inside the Tauri webview, so the reset
  // button uses a two-step inline confirmation instead (armed for 4s).
  const onResetClick = () => {
    setResetError(null)
    if (!confirmReset) {
      setConfirmReset(true)
      window.clearTimeout(resetTimer.current)
      resetTimer.current = window.setTimeout(() => setConfirmReset(false), 4000)
      return
    }
    window.clearTimeout(resetTimer.current)
    setConfirmReset(false)
    resetDb()
      .then(async () => {
        await queryClient.invalidateQueries()
      })
      .catch((e: unknown) => {
        setResetError(String(e))
        window.setTimeout(() => setResetError(null), 4000)
      })
  }

  return (
    <header className="bg-background/95 supports-[backdrop-filter]:bg-background/75 sticky top-0 z-40 flex h-12 shrink-0 items-center gap-3 border-b px-3 backdrop-blur">
      <div className="flex items-center gap-2">
        <div className="bg-primary text-primary-foreground flex size-6 items-center justify-center rounded-md">
          <Activity className="size-3.5" />
        </div>
        <span className="text-sm font-bold tracking-tight">otel-viewer</span>
      </div>

      <div className="ml-2 hidden items-center gap-1.5 lg:flex">
        <StatChip label="traces" value={String(stats?.traces ?? 0)} />
        <StatChip label="spans" value={String(stats?.spans ?? 0)} />
        <StatChip label="logs" value={String(stats?.logs ?? 0)} />
        <StatChip label="metrics" value={String(stats?.metrics ?? 0)} />
        <StatChip label="services" value={String(stats?.services ?? 0)} />
      </div>

      {health?.otlp_addr && (
        <Tooltip>
          <TooltipTrigger asChild>
            <button
              className="bg-muted/60 hidden items-center gap-1.5 rounded-md px-2 py-1 xl:flex"
              onClick={() =>
                navigator.clipboard?.writeText(`http://${health.otlp_addr}`).catch(() => {})
              }
            >
              <span className="text-muted-foreground text-[10px] tracking-wide uppercase">
                otlp
              </span>
              <span className="font-mono text-xs font-semibold">{health.otlp_addr}</span>
            </button>
          </TooltipTrigger>
          <TooltipContent>OTLP/gRPC endpoint — click to copy</TooltipContent>
        </Tooltip>
      )}

      <div className="ml-auto flex items-center gap-1.5">
        <Select value={timeRange} onValueChange={(v) => setTimeRange(v as TimeRange)}>
          <SelectTrigger size="sm" className="w-[110px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {RANGES.map((r) => (
              <SelectItem key={r.value} value={r.value}>
                {r.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>

        <Select value={preset} onValueChange={(v) => setPreset(v as Preset)}>
          <SelectTrigger size="sm" className="w-[100px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {PRESETS.map((p) => (
              <SelectItem key={p.value} value={p.value}>
                {p.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>

        <Select value={String(autoRefreshMs)} onValueChange={(v) => setAutoRefreshMs(Number(v))}>
          <SelectTrigger size="sm" className="w-[92px]" title="Auto-refresh">
            <div className="flex items-center gap-1.5">
              <Timer className="text-muted-foreground size-3" />
              <SelectValue />
            </div>
          </SelectTrigger>
          <SelectContent>
            {AUTO_REFRESH_OPTIONS.map((o) => (
              <SelectItem key={o.value} value={String(o.value)}>
                {o.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={() => queryClient.invalidateQueries()}
            >
              <RefreshCw />
            </Button>
          </TooltipTrigger>
          <TooltipContent>Refresh data</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant={confirmReset ? "destructive" : "ghost"}
              size="icon-sm"
              className={confirmReset ? undefined : "text-destructive hover:text-destructive"}
              onClick={onResetClick}
            >
              <Trash2 />
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            {confirmReset ? "Click again to delete ALL data" : "Reset database — delete all data"}
          </TooltipContent>
        </Tooltip>
        {resetError && (
          <span className="text-destructive max-w-40 truncate text-[10px]">{resetError}</span>
        )}

        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon-sm" onClick={toggleTheme}>
              {theme === "dark" ? <Sun /> : <Moon />}
            </Button>
          </TooltipTrigger>
          <TooltipContent>Toggle theme</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant={infoOpen ? "secondary" : "ghost"}
              size="icon-sm"
              onClick={() => setInfoOpen((v) => !v)}
            >
              <Info />
            </Button>
          </TooltipTrigger>
          <TooltipContent>DuckDB storage info</TooltipContent>
        </Tooltip>
      </div>

      {infoOpen && <DbInfoPopover onClose={() => setInfoOpen(false)} />}
    </header>
  )
}
