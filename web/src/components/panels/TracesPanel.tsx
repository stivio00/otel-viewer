import { useState } from "react"
import { useQuery } from "@tanstack/react-query"
import { AlertTriangle, GitBranch, ListTree } from "lucide-react"

import { fetchTraces, type TraceSummary } from "@/lib/api"
import { fmtNs, fmtNsTime, serviceColor, shortId } from "@/lib/format"
import { useDebounce } from "@/lib/hooks"
import { timeRangeStartNs, useUi } from "@/lib/store"
import { cn } from "@/lib/utils"
import { Panel, PanelGroup } from "@/components/Split"
import { EmptyState, Panel as PanelChrome } from "@/components/Panel"
import { ResizeHandle } from "@/components/Split"
import { SearchInput, ServiceSelect } from "@/components/Filters"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { ScrollArea } from "@/components/ui/scroll-area"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { TraceDetail } from "@/components/panels/TraceDetail"

export function TracesPanel() {
  const [service, setService] = useState("all")
  const [q, setQ] = useState("")
  const [errorsOnly, setErrorsOnly] = useState(false)
  const debouncedQ = useDebounce(q)
  const timeRange = useUi((s) => s.timeRange)
  const refreshTick = useUi((s) => s.refreshTick)
  const selectedTraceId = useUi((s) => s.selectedTraceId)
  const setSelectedTraceId = useUi((s) => s.setSelectedTraceId)

  const { data, isLoading } = useQuery({
    queryKey: ["traces", service, debouncedQ, errorsOnly, timeRange, refreshTick],
    queryFn: () =>
      fetchTraces({
        service: service === "all" ? undefined : service,
        q: debouncedQ || undefined,
        errors_only: errorsOnly || undefined,
        start_ns: timeRangeStartNs(timeRange),
        limit: 200,
      }),
    refetchInterval: 15_000,
  })

  const traces = data?.traces ?? []

  return (
    <PanelGroup id="otv-traces-v" orientation="vertical">
      <Panel id="traces-list" defaultSize={55} minSize={25}>
        <PanelChrome
          title="Traces"
          icon={ListTree}
          className="h-full"
          bodyClassName="flex flex-col"
          actions={
            <>
              <ServiceSelect value={service} onChange={setService} className="w-[130px]" />
              <SearchInput
                value={q}
                onChange={setQ}
                placeholder="search traces…"
                className="relative w-[180px]"
              />
              <Button
                variant={errorsOnly ? "default" : "outline"}
                size="sm"
                onClick={() => setErrorsOnly((v) => !v)}
              >
                <AlertTriangle />
                errors
              </Button>
            </>
          }
        >
          <ScrollArea className="min-h-0 flex-1">
            {isLoading ? (
              <EmptyState label="Loading…" />
            ) : traces.length === 0 ? (
              <EmptyState label="No traces match the current filters" />
            ) : (
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="pl-3">Root span</TableHead>
                    <TableHead>Services</TableHead>
                    <TableHead>When</TableHead>
                    <TableHead className="text-right">Duration</TableHead>
                    <TableHead className="text-right">Spans</TableHead>
                    <TableHead className="pr-3 text-right">Errors</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {traces.map((t) => (
                    <TraceRow
                      key={t.trace_id}
                      trace={t}
                      selected={t.trace_id === selectedTraceId}
                      onSelect={() =>
                        setSelectedTraceId(t.trace_id === selectedTraceId ? null : t.trace_id)
                      }
                    />
                  ))}
                </TableBody>
              </Table>
            )}
          </ScrollArea>
        </PanelChrome>
      </Panel>
      <ResizeHandle direction="vertical" />
      <Panel id="traces-detail" defaultSize={45} minSize={15}>
        {selectedTraceId ? (
          <TraceDetail traceId={selectedTraceId} />
        ) : (
          <PanelChrome title="Trace detail" icon={GitBranch} className="h-full">
            <EmptyState label="Select a trace to inspect its waterfall" />
          </PanelChrome>
        )}
      </Panel>
    </PanelGroup>
  )
}

function TraceRow({
  trace,
  selected,
  onSelect,
}: {
  trace: TraceSummary
  selected: boolean
  onSelect: () => void
}) {
  return (
    <TableRow
      onClick={onSelect}
      data-state={selected ? "selected" : undefined}
      className={cn("cursor-pointer", selected && "bg-accent/50")}
    >
      <TableCell className="pl-3 font-medium">
        {trace.root_name ?? shortId(trace.trace_id)}
      </TableCell>
      <TableCell>
        <div className="flex items-center gap-1">
          {(trace.services ?? []).slice(0, 4).map((s) => (
            <span
              key={s}
              className="inline-block size-2 rounded-full"
              style={{ backgroundColor: serviceColor(s) }}
              title={s}
            />
          ))}
        </div>
      </TableCell>
      <TableCell className="text-muted-foreground tabular-nums">
        {fmtNsTime(trace.start_ns)}
      </TableCell>
      <TableCell className="text-right tabular-nums">{fmtNs(trace.duration_ns)}</TableCell>
      <TableCell className="text-right tabular-nums">{trace.span_count}</TableCell>
      <TableCell className="pr-3 text-right">
        {trace.error_count > 0 ? (
          <Badge variant="destructive">{trace.error_count}</Badge>
        ) : (
          <span className="text-muted-foreground">·</span>
        )}
      </TableCell>
    </TableRow>
  )
}
