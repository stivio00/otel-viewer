import { Component, type ErrorInfo, type ReactNode } from "react"

interface Props {
  children: ReactNode
}

interface State {
  error: Error | null
}

/** Catches render crashes so the app shows an error card instead of a
 * blank white window, and offers a reload. */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null }

  static getDerivedStateFromError(error: Error): State {
    return { error }
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("[otel-viewer] render crash:", error, info.componentStack)
  }

  render() {
    if (this.state.error) {
      return (
        <div className="text-destructive flex h-full flex-col items-center justify-center gap-3 p-6 text-center">
          <p className="text-sm font-semibold">Something crashed while rendering</p>
          <pre className="bg-muted max-w-[600px] overflow-auto rounded-md p-3 text-left text-xs whitespace-pre-wrap">
            {String(this.state.error)}
          </pre>
          <button
            className="bg-primary text-primary-foreground rounded-md px-3 py-1.5 text-xs font-semibold"
            onClick={() => window.location.reload()}
          >
            Reload
          </button>
        </div>
      )
    }
    return this.props.children
  }
}
