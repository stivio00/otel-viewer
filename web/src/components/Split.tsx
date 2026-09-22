import { Group, Panel, Separator, useDefaultLayout } from "react-resizable-panels"

import { cn } from "@/lib/utils"

export { Panel }

/**
 * Panel group that persists its layout (per `id`) in localStorage.
 * A thin wrapper over react-resizable-panels v4's Group + useDefaultLayout.
 */
export function PanelGroup({
  id,
  orientation,
  children,
  className,
}: {
  id: string
  orientation: "horizontal" | "vertical"
  children: React.ReactNode
  className?: string
}) {
  const { defaultLayout, onLayoutChanged } = useDefaultLayout({ id })
  return (
    <Group
      id={id}
      orientation={orientation}
      defaultLayout={defaultLayout}
      onLayoutChanged={onLayoutChanged}
      className={cn("h-full", className)}
    >
      {children}
    </Group>
  )
}

/** Resize grip. `direction` is the group's orientation. */
export function ResizeHandle({
  direction,
  className,
}: {
  direction: "horizontal" | "vertical"
  className?: string
}) {
  return (
    <Separator
      className={cn(
        "bg-transparent relative flex items-center justify-center",
        direction === "horizontal" ? "w-1.5" : "h-1.5",
        className
      )}
    >
      <div
        className={cn(
          "bg-border hover:bg-primary rounded-full transition-colors",
          direction === "horizontal" ? "inset-y-3 w-0.5" : "inset-x-3 h-0.5"
        )}
      />
    </Separator>
  )
}
