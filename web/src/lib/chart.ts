// Shared chart-shape detection: turn arbitrary /api/query results into
// plottable shapes (time series, scatter, bars). Used by the SQL panel's
// auto-chart and by dashboard panels.

import type { ColumnInfo, QueryResponse } from "@/lib/api"

/** Shared recharts Tooltip styling (theme tokens, compact). */
export const CHART_TOOLTIP_STYLE = {
  backgroundColor: "var(--popover)",
  border: "1px solid var(--border)",
  borderRadius: 8,
  fontSize: 11,
} as const

/** Numeric value from a number or numeric-looking string (HUGEINT columns
 *  such as sum()/count() come back from the API as strings). */
export function toNum(v: unknown): number | null {
  if (typeof v === "number" && Number.isFinite(v)) return v
  if (typeof v === "string" && v !== "") {
    const n = Number(v)
    return Number.isFinite(n) ? n : null
  }
  return null
}

function epochScale(v: number): number | null {
  if (v >= 1e17) return v / 1e6 // ns
  if (v >= 1e14) return v / 1e3 // us
  if (v >= 1e11) return v // ms
  if (v >= 1e8) return v * 1e3 // s
  return null
}

/** Value to epoch-ms when it looks like a timestamp, else null. */
export function toTimeMs(v: unknown): number | null {
  if (typeof v === "number" && Number.isFinite(v)) return epochScale(v)
  if (typeof v === "string" && v !== "") {
    if (/[-:T ]/.test(v)) {
      const t = Date.parse(v)
      return Number.isNaN(t) ? null : t
    }
    const n = Number(v)
    return Number.isFinite(n) ? epochScale(n) : null
  }
  return null
}

export const TIME_NAME = /^(ts|t_|_?ts$|time|timestamp|when|date)/i

export interface PlotShape {
  xCol: string
  series: string[]
  rows: Array<Record<string, number | null>>
}

function numericNonNull(result: QueryResponse, c: ColumnInfo): number {
  let n = 0
  for (const r of result.rows) {
    if (toNum(r[c.name]) != null) n++
  }
  return n
}

/** Time-like x column + up to 8 numeric y columns (multi-line chart). */
export function detectPlot(result: QueryResponse): PlotShape | null {
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
    return numericNonNull(result, c) > 0
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
      row[name] = toNum(r[name])
    }
    rows.push(row)
  }
  if (rows.length < 2) return null
  rows.sort((a, b) => (a.x ?? 0) - (b.x ?? 0))
  return { xCol: xCol.name, series, rows }
}

export interface ScatterShape {
  xCol: string
  yCols: string[]
  rows: Array<Record<string, number | null>>
}

/** Any numeric/date x column + numeric y columns (scatter/point chart). */
export function detectScatter(result: QueryResponse): ScatterShape | null {
  if (result.rows.length < 2) return null
  const num = (c: ColumnInfo) => numericNonNull(result, c) > 0
  const xCol =
    result.columns.find((c) => TIME_NAME.test(c.name) && num(c)) ??
    result.columns.find(num)
  if (!xCol) return null
  const yCols = result.columns
    .filter((c) => c.name !== xCol.name && num(c))
    .map((c) => c.name)
    .slice(0, 4)
  if (yCols.length === 0) return null
  const rows = result.rows
    .map((r) => {
      const row: Record<string, number | null> = {}
      const x = toNum(r[xCol.name])
      row.x = x != null && x < 1e11 ? x : toTimeMs(r[xCol.name])
      for (const y of yCols) {
        row[y] = toNum(r[y])
      }
      return row
    })
    .filter((r) => r.x != null)
  return rows.length >= 2 ? { xCol: xCol.name, yCols, rows } : null
}

export interface BarShape {
  labelCol: string
  valueCols: string[]
  rows: Array<Record<string, string | number | null>>
}

/** First non-numeric column as label + numeric columns as values. */
export function detectBar(result: QueryResponse): BarShape | null {
  if (result.rows.length < 1) return null
  const numeric = new Set(
    result.columns.filter((c) => numericNonNull(result, c) > 0).map((c) => c.name)
  )
  const labelCol =
    result.columns.find((c) => !numeric.has(c.name) && result.rows.some((r) => r[c.name] != null))
      ?.name ?? result.columns[0]?.name
  if (!labelCol) return null
  const valueCols = result.columns.filter((c) => numeric.has(c.name)).map((c) => c.name)
  if (valueCols.length === 0) return null
  const rows = result.rows.map((r) => {
    const row: Record<string, string | number | null> = {
      label: String(r[labelCol] ?? ""),
    }
    for (const c of valueCols) {
      row[c] = toNum(r[c])
    }
    return row
  })
  return { labelCol, valueCols, rows }
}

export interface HistShape {
  col: string
  // {x0, x1, count}
  bins: Array<{ x0: number; x1: number; count: number }>
  min: number
  max: number
}

/** Client-side histogram of the first numeric column (Sturges bins). */
export function detectHist(result: QueryResponse): HistShape | null {
  const col = result.columns.find((c) => numericNonNull(result, c) > 0)
  if (!col) return null
  const values = result.rows
    .map((r) => toNum(r[col.name]))
    .filter((v): v is number => v != null)
  if (values.length < 3) return null
  const min = Math.min(...values)
  const max = Math.max(...values)
  if (min === max) return null
  const k = Math.max(5, Math.min(40, Math.ceil(Math.log2(values.length)) + 1))
  const w = (max - min) / k
  const bins = Array.from({ length: k }, (_, i) => ({
    x0: min + i * w,
    x1: min + (i + 1) * w,
    count: 0,
  }))
  for (const v of values) {
    const idx = Math.min(k - 1, Math.floor((v - min) / w))
    bins[idx].count++
  }
  return { col: col.name, bins, min, max }
}

export interface HeatShape {
  // rows: time (ms), y (number), weight
  cells: Array<{ x: number; y: number; weight: number }>
  xs: number[]
  ys: number[]
  maxWeight: number
}

/** time-ish x + numeric y + numeric weight columns → heatmap grid. */
export function detectHeat(result: QueryResponse): HeatShape | null {
  if (result.rows.length < 2) return null
  const numeric = new Set(
    result.columns.filter((c) => numericNonNull(result, c) > 0).map((c) => c.name)
  )
  const xCol = result.columns.find((c) => TIME_NAME.test(c.name) && numeric.has(c.name))
  const rest = result.columns.filter((c) => c.name !== xCol?.name && numeric.has(c.name))
  if (!xCol || rest.length < 2) return null
  const [yCol, wCol] = rest
  const cells: HeatShape["cells"] = []
  for (const r of result.rows) {
    const x = toTimeMs(r[xCol.name])
    const y = toNum(r[yCol.name])
    const w = toNum(r[wCol.name])
    if (x == null || y == null || w == null) continue
    cells.push({ x, y, weight: w })
  }
  if (cells.length < 2) return null
  const xs = [...new Set(cells.map((c) => c.x))].sort((a, b) => a - b)
  const ys = [...new Set(cells.map((c) => c.y))].sort((a, b) => a - b)
  const maxWeight = Math.max(...cells.map((c) => c.weight), 1)
  return { cells, xs, ys, maxWeight }
}
