import { ChevronDown } from "lucide-react";
import { useTranslation } from "react-i18next";

interface ModelSelectorProps {
  models: string[];
  value: string;
  onChange: (value: string) => void;
}

export function ModelSelector({ models, value, onChange }: ModelSelectorProps) {
  const { t } = useTranslation();
  return (
    <label className="flex flex-col gap-1 text-sm text-text-secondary">
      {t("chatSettings.model")}
      <div className="relative">
        <select
          value={value}
          onChange={(event) => onChange(event.target.value)}
          className="h-10 w-full appearance-none rounded-md border border-glass-border bg-space-deep px-3 pr-9 text-text-primary shadow-sm outline-none focus:ring-1 focus:ring-glow-blue"
        >
          {models.map((model) => (
            <option key={model} value={model} className="bg-space-deep text-text-primary">
              {model}
            </option>
          ))}
        </select>
        <ChevronDown className="pointer-events-none absolute right-3 top-1/2 size-4 -translate-y-1/2 text-text-muted" />
      </div>
    </label>
  );
}
