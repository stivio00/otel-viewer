import { create } from "zustand"

export type Preset = "default" | "traces" | "logs" | "metrics" | "sql" | "dashboards"
export type TimeRange = "5m" | "15m" | "1h" | "6h" | "24h" | "all"
export type Theme = "dark" | "light"

const DEFAULT_SQL =
  "SELECT service_name, count(*) AS spans, round(avg(duration_ms), 2) AS avg_ms\nFROM spans GROUP BY 1 ORDER BY spans DESC"

interface UiState {
  theme: Theme
  preset: Preset
  timeRange: TimeRange
  selectedTraceId: string | null
  refreshTick: number
  sqlText: string
  setTheme: (t: Theme) => void
  toggleTheme: () => void
  setPreset: (p: Preset) => void
  setTimeRange: (r: TimeRange) => void
  setSelectedTraceId: (id: string | null) => void
  refresh: () => void
  setSqlText: (t: string) => void
}

export const useUi = create<UiState>((set) => ({
  theme: (localStorage.getItem("otv-theme") as Theme) ?? "dark",
  preset: (localStorage.getItem("otv-preset") as Preset) ?? "default",
  timeRange: "all",
  selectedTraceId: null,
  refreshTick: 0,
  sqlText: localStorage.getItem("otv-sql") ?? DEFAULT_SQL,
  setTheme: (t) => {
    localStorage.setItem("otv-theme", t)
    set({ theme: t })
  },
  toggleTheme: () => {
    const next: Theme = useUi.getState().theme === "dark" ? "light" : "dark"
    localStorage.setItem("otv-theme", next)
    set({ theme: next })
  },
  setPreset: (p) => {
    localStorage.setItem("otv-preset", p)
    set({ preset: p })
  },
  setTimeRange: (r) => set({ timeRange: r }),
  setSelectedTraceId: (id) => set({ selectedTraceId: id }),
  refresh: () => set((s) => ({ refreshTick: s.refreshTick + 1 })),
  setSqlText: (t) => {
    localStorage.setItem("otv-sql", t)
    set({ sqlText: t })
  },
}))

const RANGE_MS: Record<Exclude<TimeRange, "all">, number> = {
  "5m": 5 * 60_000,
  "15m": 15 * 60_000,
  "1h": 60 * 60_000,
  "6h": 6 * 60 * 60_000,
  "24h": 24 * 60 * 60_000,
}

/** Lower bound (ns, as string) for the current time range, or undefined. */
export function timeRangeStartNs(range: TimeRange): string | undefined {
  if (range === "all") return undefined
  const now = Date.now()
  return String((now - RANGE_MS[range]) * 1_000_000)
}
