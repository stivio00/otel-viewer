import { useEffect, useState } from "react"

import { useUi } from "@/lib/store"

export function useDebounce<T>(value: T, delayMs = 300): T {
  const [debounced, setDebounced] = useState(value)
  useEffect(() => {
    const t = setTimeout(() => setDebounced(value), delayMs)
    return () => clearTimeout(t)
  }, [value, delayMs])
  return debounced
}

/**
 * Whether recharts series should animate. Animations look good on mount /
 * page switches but make the UI flicker when auto-refresh repaints every
 * few hundred ms — so they are disabled while auto-refresh is active.
 */
export function useChartAnimate(): boolean {
  return useUi((s) => s.autoRefreshMs === 0)
}
