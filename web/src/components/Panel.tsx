import type { ReactNode } from "react"
import type { LucideIcon } from "lucide-react"

import { cn } from "@/lib/utils"

export function Panel({
  title,
  icon: Icon,
  actions,
  children,
  className,
  bodyClassName,
}: {
  title: ReactNode
  icon?: LucideIcon
  actions?: ReactNode
  children: ReactNode
  className?: string
  bodyClassName?: string
}) {
  return (
    <div
      data-slot="panel"
      className={cn(
        "bg-card flex min-h-0 min-w-0 flex-col overflow-hidden rounded-lg border",
        className
      )}
    >
      <div className="flex h-9 shrink-0 items-center gap-2 border-b px-3">
        {Icon && <Icon className="text-muted-foreground size-3.5" />}
        <div className="text-foreground truncate text-xs font-semibold tracking-wide uppercase">
          {title}
        </div>
        <div className="ml-auto flex items-center gap-1.5">{actions}</div>
      </div>
      <div className={cn("min-h-0 flex-1 overflow-auto", bodyClassName)}>{children}</div>
    </div>
  )
}

export function EmptyState({ label }: { label: string }) {
  return (
    <div className="text-muted-foreground flex h-full items-center justify-center py-16 text-sm">
      {label}
    </div>
  )
}
