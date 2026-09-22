import { useState } from "react"
import { useQuery } from "@tanstack/react-query"
import { ChevronDown, Database } from "lucide-react"

import { fetchSchema, type TableInfo } from "@/lib/api"
import { useUi } from "@/lib/store"
import { cn } from "@/lib/utils"
import { Panel as PanelChrome } from "@/components/Panel"
import { Badge } from "@/components/ui/badge"
import { ScrollArea } from "@/components/ui/scroll-area"

export function SchemaPanel({ onPickTable }: { onPickTable?: (table: string) => void }) {
  const refreshTick = useUi((s) => s.refreshTick)
  const { data, isLoading } = useQuery({
    queryKey: ["schema", refreshTick],
    queryFn: fetchSchema,
    staleTime: 60_000,
  })
  const [open, setOpen] = useState<Record<string, boolean>>({})

  const tables = data?.tables ?? []

  return (
    <PanelChrome
      title="Schema"
      icon={Database}
      className="h-full"
      bodyClassName="h-full overflow-hidden"
    >
      <ScrollArea className="h-full">
        <div className="p-2">
          {isLoading ? (
            <p className="text-muted-foreground px-2 py-4 text-xs">Loading…</p>
          ) : (
            tables.map((t) => (
              <SchemaTable
                key={t.name}
                table={t}
                open={open[t.name] ?? false}
                onToggle={() =>
                  setOpen((o) => ({ ...o, [t.name]: !(o[t.name] ?? false) }))
                }
                onPickTable={onPickTable}
              />
            ))
          )}
        </div>
      </ScrollArea>
    </PanelChrome>
  )
}

function SchemaTable({
  table,
  open,
  onToggle,
  onPickTable,
}: {
  table: TableInfo
  open: boolean
  onToggle: () => void
  onPickTable?: (table: string) => void
}) {
  return (
    <div className="mb-1 overflow-hidden rounded-md border">
      <button
        onClick={onToggle}
        onDoubleClick={() => onPickTable?.(table.name)}
        className="hover:bg-accent/50 flex w-full items-center gap-1.5 px-2 py-1.5 text-left"
        title="Double-click to insert SELECT into the console"
      >
        <ChevronDown
          className={cn(
            "text-muted-foreground size-3 transition-transform",
            !open && "-rotate-90"
          )}
        />
        <span className="font-mono text-xs font-medium">{table.name}</span>
        <Badge variant="secondary" className="ml-auto text-[10px] tabular-nums">
          {table.row_count}
        </Badge>
      </button>
      {open && (
        <div className="border-t">
          {table.columns.map((c) => (
            <div
              key={c.name}
              className="flex items-baseline justify-between gap-2 px-2 py-0.5 pl-6 text-[11px]"
            >
              <span className="truncate font-mono">{c.name}</span>
              <span className="text-muted-foreground shrink-0 font-mono text-[10px]">
                {c.type}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
