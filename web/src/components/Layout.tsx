import { Panel, PanelGroup } from "@/components/Split"
import { useUi } from "@/lib/store"
import { ResizeHandle } from "@/components/Split"
import { TracesPanel } from "@/components/panels/TracesPanel"
import { LogsPanel } from "@/components/panels/LogsPanel"
import { MetricsPanel } from "@/components/panels/MetricsPanel"
import { SqlPreset } from "@/components/panels/SqlPanel"

export function Layout() {
  const preset = useUi((s) => s.preset)

  switch (preset) {
    case "traces":
      return (
        <div className="h-full">
          <TracesPanel />
        </div>
      )
    case "logs":
      return (
        <div className="h-full">
          <LogsPanel />
        </div>
      )
    case "metrics":
      return (
        <div className="h-full">
          <MetricsPanel />
        </div>
      )
    case "sql":
      return (
        <div className="h-full">
          <SqlPreset />
        </div>
      )
    default:
      return (
        <PanelGroup id="otv-default-h" orientation="horizontal">
          <Panel id="def-traces" defaultSize={50} minSize={25}>
            <TracesPanel />
          </Panel>
          <ResizeHandle direction="horizontal" />
          <Panel id="def-logs" defaultSize={50} minSize={25}>
            <LogsPanel />
          </Panel>
        </PanelGroup>
      )
  }
}
