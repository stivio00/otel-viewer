import { format } from "date-fns"

/** Nanoseconds (possibly a string from the API) to milliseconds. */
export function nsToMs(ns: string | number | null | undefined): number {
  if (ns == null) return 0
  return Number(ns) / 1e6
}

/** Human-readable duration from nanoseconds. */
export function fmtNs(ns: string | number | null | undefined): string {
  const ms = nsToMs(ns)
  if (ms === 0) return "0ms"
  if (ms < 1) return `${(ms * 1000).toFixed(0)}µs`
  if (ms < 1000) return `${ms < 10 ? ms.toFixed(2) : ms.toFixed(0)}ms`
  return `${(ms / 1000).toFixed(2)}s`
}

/** Absolute timestamp from nanoseconds. */
export function fmtNsTime(ns: string | number | null | undefined): string {
  if (ns == null) return "-"
  const ms = nsToMs(ns)
  if (ms <= 0) return "-"
  return format(new Date(ms), "HH:mm:ss.SSS")
}

export function fmtNsDate(ns: string | number | null | undefined): string {
  if (ns == null) return "-"
  const ms = nsToMs(ns)
  if (ms <= 0) return "-"
  return format(new Date(ms), "MM-dd HH:mm:ss")
}

/** First 8 chars of a trace/span id. */
export function shortId(id: string | null | undefined): string {
  if (!id) return "-"
  return id.slice(0, 8)
}

export function severityVariant(
  sev: string | null | undefined
): "destructive" | "default" | "secondary" | "outline" {
  const s = (sev ?? "").toUpperCase()
  if (s === "ERROR" || s === "FATAL") return "destructive"
  if (s === "WARN" || s === "WARNING") return "default"
  if (s === "INFO") return "secondary"
  return "outline"
}

export function severityClass(sev: string | null | undefined): string {
  const s = (sev ?? "").toUpperCase()
  if (s === "ERROR" || s === "FATAL") return "text-red-400"
  if (s === "WARN" || s === "WARNING") return "text-amber-400"
  if (s === "INFO") return "text-sky-400"
  return "text-muted-foreground"
}

const SPAN_KINDS = [
  "unspecified",
  "internal",
  "server",
  "client",
  "producer",
  "consumer",
]

export function spanKindName(kind: number): string {
  return SPAN_KINDS[kind] ?? `kind${kind}`
}

export function statusBadge(code: number): { label: string; error: boolean } {
  if (code === 2) return { label: "ERROR", error: true }
  if (code === 1) return { label: "OK", error: false }
  return { label: "UNSET", error: false }
}

/** Deterministic color for a service name. */
const SERVICE_HUES = [212, 262, 292, 322, 142, 12, 42, 172]
export function serviceColor(service: string): string {
  let h = 0
  for (let i = 0; i < service.length; i++) h = (h * 31 + service.charCodeAt(i)) >>> 0
  const hue = SERVICE_HUES[h % SERVICE_HUES.length]
  const sat = 70 + (h % 3) * 8
  return `hsl(${hue} ${sat}% 60%)`
}

export function prettyJson(v: unknown): string {
  if (v == null) return ""
  if (typeof v === "string") {
    try {
      return JSON.stringify(JSON.parse(v), null, 2)
    } catch {
      return v
    }
  }
  return JSON.stringify(v, null, 2)
}

/** Parse a stored-JSON field (attributes etc.) into an object, or null. */
export function parseJsonField(v: unknown): Record<string, unknown> | null {
  if (v == null) return null
  if (typeof v === "string") {
    if (v === "") return null
    try {
      const parsed = JSON.parse(v)
      return typeof parsed === "object" && parsed !== null
        ? (parsed as Record<string, unknown>)
        : null
    } catch {
      return null
    }
  }
  if (typeof v === "object") return v as Record<string, unknown>
  return null
}

export function num(v: unknown): string {
  if (v == null) return "-"
  if (typeof v === "number") return Number.isInteger(v) ? String(v) : v.toFixed(3)
  if (typeof v === "boolean") return v ? "true" : "false"
  if (typeof v === "string") return v
  if (typeof v === "bigint") return v.toString()
  return JSON.stringify(v)
}
