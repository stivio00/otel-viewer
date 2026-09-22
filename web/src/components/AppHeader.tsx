import { useQuery } from "@tanstack/react-query"
import { Activity, Moon, RefreshCw, Sun } from "lucide-react"

import { fetchStats } from "@/lib/api"
import { useUi, type Preset, type TimeRange } from "@/lib/store"
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

export function AppHeader() {
  const { data: stats } = useQuery({
    queryKey: ["stats"],
    queryFn: fetchStats,
    refetchInterval: 5_000,
  })
  const theme = useUi((s) => s.theme)
  const toggleTheme = useUi((s) => s.toggleTheme)
  const preset = useUi((s) => s.preset)
  const setPreset = useUi((s) => s.setPreset)
  const timeRange = useUi((s) => s.timeRange)
  const setTimeRange = useUi((s) => s.setTimeRange)
  const refresh = useUi((s) => s.refresh)

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

        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon-sm" onClick={refresh}>
              <RefreshCw />
            </Button>
          </TooltipTrigger>
          <TooltipContent>Refresh data</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="icon-sm" onClick={toggleTheme}>
              {theme === "dark" ? <Sun /> : <Moon />}
            </Button>
          </TooltipTrigger>
          <TooltipContent>Toggle theme</TooltipContent>
        </Tooltip>
      </div>
    </header>
  )
}
