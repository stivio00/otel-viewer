import { useState } from "react"
import { useMutation } from "@tanstack/react-query"
import CodeMirror from "@uiw/react-codemirror"
import { sql } from "@codemirror/lang-sql"
import { oneDark } from "@codemirror/theme-one-dark"
import { Play, Table2, Terminal } from "lucide-react"

import { runQuery, type QueryResponse } from "@/lib/api"
import { num } from "@/lib/format"
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
]

export function SqlPanel() {
  const theme = useUi((s) => s.theme)
  const text = useUi((s) => s.sqlText)
  const setText = useUi((s) => s.setSqlText)
  const [result, setResult] = useState<QueryResponse | null>(null)

  const mutation = useMutation({
    mutationFn: () => runQuery(text),
    onSuccess: setResult,
    onError: () => setResult(null),
  })

  const run = () => mutation.mutate()

  const error = mutation.error instanceof Error ? mutation.error.message : null

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
            <ScrollArea className="min-h-0 flex-1">
              <Table>
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
