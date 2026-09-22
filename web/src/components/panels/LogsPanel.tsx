import { useState } from "react"
import { useQuery } from "@tanstack/react-query"
import { FileText, GitBranch, ScrollText } from "lucide-react"

import { fetchLogs, type LogDto } from "@/lib/api"
import { fmtNsTime, parseJsonField, prettyJson, severityClass } from "@/lib/format"
import { useDebounce } from "@/lib/hooks"
import { timeRangeStartNs, useUi } from "@/lib/store"
import { cn } from "@/lib/utils"
import { Panel, PanelGroup } from "@/components/Split"
import { EmptyState, Panel as PanelChrome } from "@/components/Panel"
import { ResizeHandle } from "@/components/Split"
import { SearchInput, ServiceSelect } from "@/components/Filters"
import { JsonGrid } from "@/components/panels/TraceDetail"
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
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"

const SEVERITIES = [
  { value: "all", label: "all levels" },
  { value: "debug", label: "debug+" },
  { value: "info", label: "info+" },
  { value: "warn", label: "warn+" },
  { value: "error", label: "error+" },
]

export function LogsPanel() {
  const [service, setService] = useState("all")
  const [severity, setSeverity] = useState("all")
  const [q, setQ] = useState("")
  const debouncedQ = useDebounce(q)
  const timeRange = useUi((s) => s.timeRange)
  const refreshTick = useUi((s) => s.refreshTick)
  const setPreset = useUi((s) => s.setPreset)
  const setSelectedTraceId = useUi((s) => s.setSelectedTraceId)
  const [selected, setSelected] = useState<LogDto | null>(null)

  const { data, isLoading } = useQuery({
    queryKey: ["logs", service, severity, debouncedQ, timeRange, refreshTick],
    queryFn: () =>
      fetchLogs({
        service: service === "all" ? undefined : service,
        severity: severity === "all" ? undefined : severity,
        q: debouncedQ || undefined,
        start_ns: timeRangeStartNs(timeRange),
        limit: 300,
      }),
    refetchInterval: 15_000,
  })

  const logs = data?.logs ?? []

  const viewTrace = (log: LogDto) => {
    if (!log.trace_id) return
    setSelectedTraceId(log.trace_id)
    setPreset("traces")
  }

  return (
    <PanelGroup id="otv-logs-v" orientation="vertical">
      <Panel id="logs-list" defaultSize={60} minSize={25}>
        <PanelChrome
          title="Logs"
          icon={ScrollText}
          className="h-full"
          bodyClassName="flex flex-col"
          actions={
            <>
              <ServiceSelect value={service} onChange={setService} className="w-[130px]" />
              <Select value={severity} onValueChange={setSeverity}>
                <SelectTrigger size="sm" className="w-[110px]">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {SEVERITIES.map((s) => (
                    <SelectItem key={s.value} value={s.value}>
                      {s.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <SearchInput
                value={q}
                onChange={setQ}
                placeholder="search logs…"
                className="relative w-[180px]"
              />
            </>
          }
        >
          <ScrollArea className="min-h-0 flex-1">
            {isLoading ? (
              <EmptyState label="Loading…" />
            ) : logs.length === 0 ? (
              <EmptyState label="No logs match the current filters" />
            ) : (
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="pl-3">Time</TableHead>
                    <TableHead>Level</TableHead>
                    <TableHead>Service</TableHead>
                    <TableHead>Body</TableHead>
                    <TableHead className="pr-3">Trace</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {logs.map((log, i) => (
                    <TableRow
                      key={`${log.time_ns}-${i}`}
                      onClick={() => setSelected(selected === log ? null : log)}
                      className={cn("cursor-pointer", selected === log && "bg-accent/50")}
                    >
                      <TableCell className="text-muted-foreground pl-3 tabular-nums">
                        {fmtNsTime(log.time_ns)}
                      </TableCell>
                      <TableCell className={cn("font-semibold text-xs", severityClass(log.severity_text))}>
                        {log.severity_text ?? log.severity_number}
                      </TableCell>
                      <TableCell className="text-xs">{log.service_name}</TableCell>
                      <TableCell className="max-w-[420px] truncate font-mono text-xs">
                        {bodyText(log)}
                      </TableCell>
                      <TableCell className="pr-3">
                        {log.trace_id ? (
                          <span
                            className="text-primary cursor-pointer font-mono text-xs underline-offset-2 hover:underline"
                            onClick={(e) => {
                              e.stopPropagation()
                              viewTrace(log)
                            }}
                          >
                            {log.trace_id.slice(0, 8)}…
                          </span>
                        ) : (
                          <span className="text-muted-foreground">·</span>
                        )}
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
      <Panel id="logs-detail" defaultSize={40} minSize={15}>
        {selected ? (
          <LogDetail log={selected} onTrace={() => viewTrace(selected)} />
        ) : (
          <PanelChrome title="Log detail" icon={FileText} className="h-full">
            <EmptyState label="Select a log record to inspect it" />
          </PanelChrome>
        )}
      </Panel>
    </PanelGroup>
  )
}

function bodyText(log: LogDto): string {
  const b = log.body
  if (b == null) return ""
  if (typeof b === "string") return b
  return JSON.stringify(b)
}

function LogDetail({ log, onTrace }: { log: LogDto; onTrace: () => void }) {
  const attrs = parseJsonField(log.attributes)
  const resource = parseJsonField(log.resource_attributes)
  return (
    <PanelChrome
      title="Log detail"
      icon={FileText}
      className="h-full"
      bodyClassName="overflow-auto"
      actions={
        log.trace_id && (
          <Button variant="outline" size="sm" onClick={onTrace}>
            <GitBranch />
            view trace
          </Button>
        )
      }
    >
      <div className="space-y-3 px-3 py-2">
        <div className="flex flex-wrap items-center gap-2 text-xs">
          <span className={cn("font-bold", severityClass(log.severity_text))}>
            {log.severity_text ?? `sev ${log.severity_number}`}
          </span>
          <span className="text-muted-foreground tabular-nums">{fmtNsTime(log.time_ns)}</span>
          <span className="rounded bg-muted px-1.5 py-0.5 font-mono text-[10px]">
            {log.service_name}
          </span>
          {log.scope_name && (
            <span className="text-muted-foreground font-mono text-[10px]">
              {log.scope_name}
              {log.scope_version ? `@${log.scope_version}` : ""}
            </span>
          )}
        </div>

        <div className="rounded-md border bg-muted/20 p-3">
          <pre className="wrap-anywhere overflow-x-auto font-mono text-xs leading-relaxed">
            {prettyJson(log.body)}
          </pre>
        </div>

        {log.event_name && (
          <p className="text-muted-foreground text-xs">
            event name: <span className="font-mono">{log.event_name}</span>
          </p>
        )}

        <Tabs defaultValue="attributes">
          <TabsList className="mb-2">
            <TabsTrigger value="attributes">Attributes</TabsTrigger>
            <TabsTrigger value="resource">Resource</TabsTrigger>
            <TabsTrigger value="raw">Raw</TabsTrigger>
          </TabsList>
          <TabsContent value="attributes">
            <JsonGrid data={attrs} empty="No attributes" />
          </TabsContent>
          <TabsContent value="resource">
            <JsonGrid data={resource} empty="No resource attributes" />
          </TabsContent>
          <TabsContent value="raw">
            <pre className="text-[11px] leading-relaxed">{prettyJson(log)}</pre>
          </TabsContent>
        </Tabs>
      </div>
    </PanelChrome>
  )
}
