import { useQuery } from "@tanstack/react-query"
import { Search } from "lucide-react"

import { fetchServices } from "@/lib/api"
import { Input } from "@/components/ui/input"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"

export function ServiceSelect({
  value,
  onChange,
  className,
}: {
  value: string
  onChange: (v: string) => void
  className?: string
}) {
  const { data } = useQuery({
    queryKey: ["services"],
    queryFn: fetchServices,
    staleTime: 30_000,
  })
  const services = data?.services ?? []
  return (
    <Select value={value} onValueChange={onChange}>
      <SelectTrigger size="sm" className={className}>
        <SelectValue placeholder="service" />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="all">all services</SelectItem>
        {services.map((s) => (
          <SelectItem key={s.name} value={s.name}>
            {s.name}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}

export function SearchInput({
  value,
  onChange,
  placeholder,
  className,
}: {
  value: string
  onChange: (v: string) => void
  placeholder?: string
  className?: string
}) {
  return (
    <div className={className}>
      <Search className="text-muted-foreground absolute left-2 top-1/2 size-3.5 -translate-y-1/2" />
      <Input
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="h-8 pl-7 text-xs"
      />
    </div>
  )
}
