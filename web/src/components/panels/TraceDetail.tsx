import { useMemo, useState } from "react"
import { useQuery } from "@tanstack/react-query"
import { GitBranch } from "lucide-react"

import { fetchTraceDetail, type SpanDto } from "@/lib/api"
import {
  fmtNs,
  fmtNsTime,
  parseJsonField,
  prettyJson,
  serviceColor,
  spanKindName,
  statusBadge,
} from "@/lib/format"
import { useUi } from "@/lib/store"
import { cn } from "@/lib/utils"
import { EmptyState, Panel as PanelChrome } from "@/components/Panel"
import { Badge } from "@/components/ui/badge"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"

interface LaidOutSpan {
  span: SpanDto
  depth: number
  leftPct: number
  widthPct: number
}

export function TraceDetail({ traceId }: { traceId: string }) {
  const refreshTick = useUi((s) => s.refreshTick)
  const { data: trace, isLoading, error } = useQuery({
    queryKey: ["trace-detail", traceId, refreshTick],
    queryFn: () => fetchTraceDetail(traceId),
  })
  const [selectedSpanId, setSelectedSpanId] = useState<string | null>(null)

  const layout = useMemo(() => {
    if (!trace) return { spans: [] as LaidOutSpan[], totalNs: 0 }
    const spans = [...trace.spans].sort((a, b) => Number(a.start_ns) - Number(b.start_ns))
    const byId = new Map(spans.map((s) => [s.span_id, s]))
    const depthOf = (s: SpanDto, guard = 0): number => {
      if (!s.parent_span_id || guard > 64) return 0
      const p = byId.get(s.parent_span_id)
      return p ? depthOf(p, guard + 1) + 1 : 0
    }
    const minStart = spans.length ? Number(spans[0].start_ns) : 0
    const maxEnd = spans.reduce((m, s) => Math.max(m, Number(s.end_ns)), minStart)
    const total = Math.max(maxEnd - minStart, 1)
    return {
      totalNs: total,
      spans: spans.map((s) => ({
        span: s,
        depth: depthOf(s),
        leftPct: ((Number(s.start_ns) - minStart) / total) * 100,
        widthPct: Math.max((Number(s.end_ns) - Number(s.start_ns)) / total * 100, 0.5),
      })),
    }
  }, [trace])

  if (isLoading) {
    return (
      <PanelChrome title="Trace detail" icon={GitBranch} className="h-full">
        <EmptyState label="Loading…" />
      </PanelChrome>
    )
  }
  if (error || !trace) {
    return (
      <PanelChrome title="Trace detail" icon={GitBranch} className="h-full">
        <EmptyState label="Trace not found (it may have been filtered by the time range)" />
      </PanelChrome>
    )
  }

  const selected = layout.spans.find((l) => l.span.span_id === selectedSpanId) ?? null

  return (
    <PanelChrome
      title={
        <span className="font-mono normal-case">
          {trace.spans.find((s) => !s.parent_span_id)?.span_name ?? trace.trace_id.slice(0, 16)}
        </span>
      }
      icon={GitBranch}
      className="h-full"
      bodyClassName="flex flex-col"
      actions={
        <>
          <Badge variant="secondary" className="font-mono">
            {trace.trace_id.slice(0, 16)}
          </Badge>
          <span className="text-muted-foreground text-xs tabular-nums">
            {fmtNs(trace.duration_ns)} · {trace.span_count} spans
            {trace.error_count > 0 && (
              <span className="text-destructive"> · {trace.error_count} errors</span>
            )}
          </span>
        </>
      }
    >
      <div className="flex min-h-0 flex-1 flex-col">
        <ScrollArea className="max-h-[45%] min-h-0 shrink-0 border-b">
          <div className="relative min-w-[640px] px-3 py-2">
            <TimeRuler totalNs={layout.totalNs} />
            {/* Background time grid: same grid template as the span rows so
                the quarter lines land exactly on the bar track. */}
            <div aria-hidden className="pointer-events-none absolute inset-0 z-0 px-3 py-2">
              <div className="grid h-full grid-cols-[minmax(180px,32%)_1fr] gap-3 px-1">
                <div />
                <div className="relative">
                  {[0, 25, 50, 75, 100].map((pct) => (
                    <div
                      key={pct}
                      className="bg-border/70 absolute inset-y-0 w-px"
                      style={{ left: `${pct}%` }}
                    />
                  ))}
                </div>
              </div>
            </div>
            {layout.spans.map(({ span, depth, leftPct, widthPct }) => {
              const st = statusBadge(span.status_code)
              const active = span.span_id === selectedSpanId
              return (
                <button
                  key={span.span_id}
                  onClick={() => setSelectedSpanId(active ? null : span.span_id)}
                  className={cn(
                    "group grid w-full grid-cols-[minmax(180px,32%)_1fr] items-center gap-3 rounded px-1 py-0.5 text-left",
                    active && "bg-accent/60 rounded-sm"
                  )}
                >
                  <div className="flex min-w-0 items-center gap-1.5" style={{ paddingLeft: depth * 14 }}>
                    <span
                      className="inline-block size-2 shrink-0 rounded-full"
                      style={{ backgroundColor: serviceColor(span.service_name) }}
                    />
                    <span className="truncate text-xs font-medium">{span.span_name}</span>
                    {st.error && <span className="text-destructive text-[10px]">✕</span>}
                  </div>
                  <div className="relative h-4">
                    <div
                      className={cn(
                        "absolute inset-y-1 rounded-[3px]",
                        st.error ? "bg-red-500/80" : "opacity-80 group-hover:opacity-100"
                      )}
                      style={{
                        left: `${leftPct}%`,
                        width: `${widthPct}%`,
                        backgroundColor: st.error ? undefined : serviceColor(span.service_name),
                      }}
                      title={`${span.span_name} — ${fmtNs(Number(span.end_ns) - Number(span.start_ns))}`}
                    />
                  </div>
                </button>
              )
            })}
          </div>
        </ScrollArea>

        {selected ? (
          <SpanInfo span={selected.span} />
        ) : (
          <div className="text-muted-foreground flex items-center justify-center py-6 text-xs">
            Click a span row for its attributes, events, and links
          </div>
        )}
      </div>
    </PanelChrome>
  )
}

/** Sticky ruler above the waterfall: offset labels at quarter ticks. */
function TimeRuler({ totalNs }: { totalNs: number }) {
  const ticks = [0, 25, 50, 75, 100]
  return (
    <div className="bg-card/95 sticky top-0 z-10 grid grid-cols-[minmax(180px,32%)_1fr] items-center gap-3 px-1 py-1 backdrop-blur">
      <span className="text-muted-foreground text-[10px] tracking-wide uppercase">span</span>
      <div className="relative h-3">
        {ticks.map((pct) => (
          <span
            key={pct}
            className="text-muted-foreground absolute top-0 text-[9px] tabular-nums"
            style={{
              left: `${pct}%`,
              transform:
                pct === 0 ? "none" : pct === 100 ? "translateX(-100%)" : "translateX(-50%)",
            }}
          >
            +{fmtNs(Math.round((totalNs * pct) / 100))}
          </span>
        ))}
      </div>
    </div>
  )
}

function SpanInfo({ span }: { span: SpanDto }) {
  const st = statusBadge(span.status_code)
  const attrs = parseJsonField(span.attributes)
  const resource = parseJsonField(span.resource_attributes)
  const events = parseJsonField(span.events)
  const links = parseJsonField(span.links)
  const eventsList = Array.isArray(events) ? events : events ? [events] : []

  return (
    <div className="min-h-0 flex-1 overflow-auto px-3 py-2">
      <div className="mb-2 flex flex-wrap items-center gap-2 text-xs">
        <span className="font-semibold">{span.span_name}</span>
        <Badge variant="outline">{spanKindName(span.span_kind)}</Badge>
        <Badge variant={st.error ? "destructive" : "secondary"}>
          {st.error ? st.label : st.label}
        </Badge>
        <span
          className="rounded px-1.5 py-0.5 font-mono text-[10px]"
          style={{ backgroundColor: `${serviceColor(span.service_name)}22` }}
        >
          {span.service_name}
        </span>
        <span className="text-muted-foreground tabular-nums">
          {fmtNsTime(span.start_ns)} → {fmtNsTime(span.end_ns)} ({span.duration_ms.toFixed(2)}ms)
        </span>
      </div>
      <Tabs defaultValue="attributes">
        <TabsList className="mb-2">
          <TabsTrigger value="attributes">Attributes</TabsTrigger>
          <TabsTrigger value="events">Events ({eventsList.length})</TabsTrigger>
          <TabsTrigger value="links">Links</TabsTrigger>
          <TabsTrigger value="resource">Resource</TabsTrigger>
        </TabsList>
        <TabsContent value="attributes">
          <JsonGrid data={attrs} empty="No span attributes" />
        </TabsContent>
        <TabsContent value="events">
          {eventsList.length === 0 ? (
            <p className="text-muted-foreground text-xs">No events</p>
          ) : (
            <div className="space-y-2">
              {eventsList.map((e, i) => (
                <div key={i} className="rounded-md border p-2">
                  <div className="mb-1 flex items-center gap-2 text-xs font-medium">
                    {String((e as Record<string, unknown>).name ?? `event ${i}`)}
                  </div>
                  <pre className="text-muted-foreground overflow-x-auto text-[11px] leading-relaxed">
                    {prettyJson((e as Record<string, unknown>).attributes ?? e)}
                  </pre>
                </div>
              ))}
            </div>
          )}
        </TabsContent>
        <TabsContent value="links">
          {links == null || (Array.isArray(links) && links.length === 0) ? (
            <p className="text-muted-foreground text-xs">No links</p>
          ) : (
            <pre className="text-[11px] leading-relaxed">{prettyJson(links)}</pre>
          )}
        </TabsContent>
        <TabsContent value="resource">
          <JsonGrid data={resource} empty="No resource attributes" />
        </TabsContent>
      </Tabs>
      {span.status_message && (
        <p className="text-destructive mt-2 text-xs">Status: {span.status_message}</p>
      )}
    </div>
  )
}

export function JsonGrid({
  data,
  empty,
}: {
  data: Record<string, unknown> | null
  empty: string
}) {
  if (!data || Object.keys(data).length === 0) {
    return <p className="text-muted-foreground text-xs">{empty}</p>
  }
  return (
    <div className="overflow-hidden rounded-md border">
      {Object.entries(data).map(([k, v], i) => (
        <div
          key={k}
          className={cn(
            "grid grid-cols-[minmax(140px,35%)_1fr] gap-2 px-2 py-1 text-[11px]",
            i % 2 === 1 && "bg-muted/30"
          )}
        >
          <span className="text-muted-foreground truncate font-mono" title={k}>
            {k}
          </span>
          <span className="truncate font-mono" title={String(v)}>
            {typeof v === "object" ? JSON.stringify(v) : String(v)}
          </span>
        </div>
      ))}
    </div>
  )
}
