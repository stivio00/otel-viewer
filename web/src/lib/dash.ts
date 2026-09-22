// Dashboard document types and SQL template rendering.
//
// A dashboard is a YAML document (see src/dashboards/*.yml). Panel SQL is a
// template: `$from_ns` / `$to_ns` plus one token per dashboard input. An
// empty input selection substitutes NULL so the usual
// `('$x' IS NULL OR … = '$x')` branch disables the filter.

export interface DashboardInput {
  name: string
  label?: string
  // service → distinct service_name; attribute → distinct values of a JSON
  // attribute key on one of the telemetry tables; select → fixed choices.
  type: "service" | "attribute" | "select"
  table?: "spans" | "logs" | "metrics"
  key?: string
  choices?: Array<{ value: string; label?: string }>
  default?: string
}

export type PanelType =
  | "line"
  | "points"
  | "bar"
  | "histogram"
  | "dial"
  | "stat"
  | "heatmap"

export interface DashboardPanel {
  id: string
  title?: string
  type: PanelType
  unit?: string
  span?: number
  sql?: string
  max?: number
}

export interface DashboardSpec {
  name?: string
  title?: string
  description?: string
  inputs?: DashboardInput[]
  panels?: DashboardPanel[]
}

export interface DashboardMetaDto {
  id: string
  source: "builtin" | "user"
  name: string
  title: string
  description: string
}

export interface DashboardDto extends DashboardMetaDto {
  spec: DashboardSpec
}

/** Attribute input → the JSON column + path for its table. */
export function attributeTarget(input: DashboardInput): { column: string; path: string } | null {
  if (input.type !== "attribute" || !input.key) return null
  switch (input.table) {
    case "spans":
      return { column: "span_attributes", path: input.key }
    case "logs":
      return { column: "log_attributes", path: input.key }
    case "metrics":
      return { column: "series_attributes", path: input.key }
    default:
      return null
  }
}

/** SQL that lists the distinct options for an attribute input. */
export function attributeOptionsSql(input: DashboardInput): string | null {
  const t = attributeTarget(input)
  if (!t) return null
  const table =
    input.table === "spans" ? "spans" : input.table === "logs" ? "log_records" : "metric_points"
  return `SELECT DISTINCT json_extract_string(${t.column}, '$."${t.path}"') AS v
FROM ${table}
WHERE ${t.column} IS NOT NULL`
}

function sqlStringLiteral(s: string): string {
  return `'${s.replace(/'/g, "''")}'`
}

export interface RenderVars {
  fromNs: number
  toNs: number
  inputs: Record<string, string>
}

/**
 * Substitute `$from_ns`, `$to_ns` and `$<input>` tokens in a panel SQL
 * template. Quoted tokens (`'$x'`) are replaced including their quotes so an
 * empty input becomes a bare SQL NULL (not the string 'NULL'); the usual
 * `('$x' IS NULL OR … = '$x')` branch then disables the filter. Values are
 * escaped as SQL string literals.
 */
export function renderSql(template: string, vars: RenderVars): string {
  // Quoted input tokens first: '$x' -> NULL | 'literal'
  let out = template.replace(/'\$([a-zA-Z_][a-zA-Z0-9_]*)'/g, (tok, name: string) => {
    if (name === "from_ns" || name === "to_ns") return tok
    const v = vars.inputs[name]
    if (v === undefined) return tok
    return v === "" ? "NULL" : sqlStringLiteral(v)
  })
  // Then bare tokens (outside quotes): numbers for from/to, literals for
  // inputs. Quoted occurrences were handled above (or are unknown tokens —
  // leave those alone).
  return out.replace(/(?<!')\$(from_ns|to_ns|[a-zA-Z_][a-zA-Z0-9_]*)(?!')/g, (tok, name: string) => {
    if (name === "from_ns") return String(Math.floor(vars.fromNs))
    if (name === "to_ns") return String(Math.floor(vars.toNs))
    const v = vars.inputs[name]
    if (v === undefined) return tok
    return v === "" ? "NULL" : sqlStringLiteral(v)
  })
}
