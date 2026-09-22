import { useEffect } from "react"

import { useUi } from "@/lib/store"
import { AppHeader } from "@/components/AppHeader"
import { ErrorBoundary } from "@/components/ErrorBoundary"
import { Layout } from "@/components/Layout"

export default function App() {
  const theme = useUi((s) => s.theme)

  useEffect(() => {
    document.documentElement.classList.toggle("dark", theme === "dark")
  }, [theme])

  return (
    <div className="bg-background text-foreground flex h-full flex-col overflow-hidden">
      <AppHeader />
      <main className="min-h-0 flex-1 p-2">
        <ErrorBoundary>
          <Layout />
        </ErrorBoundary>
      </main>
    </div>
  )
}
