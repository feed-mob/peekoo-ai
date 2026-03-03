import type { ProviderCatalog } from "@/types/agent-settings";
import { ChevronDown } from "lucide-react";

interface ProviderSelectorProps {
  providers: ProviderCatalog[];
  value: string;
  onChange: (value: string) => void;
}

export function ProviderSelector({ providers, value, onChange }: ProviderSelectorProps) {
  return (
    <label className="flex flex-col gap-1 text-sm text-text-secondary">
      Provider
      <div className="relative">
        <select
          value={value}
          onChange={(event) => onChange(event.target.value)}
          className="h-10 w-full appearance-none rounded-md border border-glass-border bg-space-deep px-3 pr-9 text-text-primary shadow-sm outline-none focus:ring-1 focus:ring-glow-blue"
        >
          {providers.map((provider) => (
            <option key={provider.id} value={provider.id} className="bg-space-deep text-text-primary">
              {provider.name}
            </option>
          ))}
        </select>
        <ChevronDown className="pointer-events-none absolute right-3 top-1/2 size-4 -translate-y-1/2 text-text-muted" />
      </div>
    </label>
  );
}
