import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useState } from "react";
import { SlidersHorizontal, MoonStar } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import {
  type PluginConfigField,
  pluginConfigFieldSchema,
} from "@/types/plugin";

type ConfigValues = Record<string, unknown>;

interface PluginConfigPanelProps {
  pluginKey: string;
}

export function PluginConfigPanel({ pluginKey }: PluginConfigPanelProps) {
  const [fields, setFields] = useState<PluginConfigField[]>([]);
  const [values, setValues] = useState<ConfigValues>({});
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isDnd, setIsDnd] = useState(false);

  const hasFields = fields.length > 0;

  useEffect(() => {
    let cancelled = false;

    const load = async () => {
      setIsLoading(true);
      setError(null);
      try {
        const [rawFields, rawValues, rawDnd] = await Promise.all([
          invoke("plugin_config_schema", { pluginKey }),
          invoke<Record<string, unknown>>("plugin_config_get", { pluginKey }),
          invoke<boolean>("dnd_get"),
        ]);

        if (cancelled) {
          return;
        }

        setFields(pluginConfigFieldSchema.array().parse(rawFields));
        setValues(rawValues ?? {});
        setIsDnd(Boolean(rawDnd));
      } catch (err) {
        if (!cancelled) {
          setError(String(err));
        }
      } finally {
        if (!cancelled) {
          setIsLoading(false);
        }
      }
    };

    void load();

    return () => {
      cancelled = true;
    };
  }, [pluginKey]);

  const sortedFields = useMemo(
    () => [...fields].sort((a, b) => a.label.localeCompare(b.label)),
    [fields],
  );

  const updateValue = (key: string, value: unknown) => {
    setValues((current) => ({ ...current, [key]: value }));
  };

  const save = async () => {
    setIsSaving(true);
    setError(null);
    try {
      await Promise.all(
        sortedFields.map((field) =>
          invoke("plugin_config_set", {
            pluginKey,
            key: field.key,
            value: values[field.key] ?? field.default,
          }),
        ),
      );
    } catch (err) {
      setError(String(err));
    } finally {
      setIsSaving(false);
    }
  };

  const toggleDnd = async () => {
    const nextValue = !isDnd;
    setIsDnd(nextValue);
    try {
      await invoke("dnd_set", { active: nextValue });
    } catch (err) {
      setIsDnd(!nextValue);
      setError(String(err));
    }
  };

  if (isLoading) {
    return <div className="text-xs text-text-muted">Loading plugin settings...</div>;
  }

  if (!hasFields && !error) {
    return null;
  }

  return (
    <div className="mt-4 space-y-3 rounded-2xl border border-glass-border bg-space-deep/40 p-4">
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="flex items-center gap-2 text-xs uppercase tracking-[0.16em] text-text-muted">
            <SlidersHorizontal size={12} /> Runtime settings
          </p>
          <p className="mt-1 text-sm text-text-secondary">
            Configure reminder timing and global quiet hours.
          </p>
        </div>
        <Button size="sm" variant="outline" onClick={toggleDnd}>
          <MoonStar size={14} />
          {isDnd ? "DND on" : "DND off"}
        </Button>
      </div>

      {error ? (
        <div className="rounded-xl border border-danger/30 bg-danger/10 px-3 py-2 text-xs text-danger">
          {error}
        </div>
      ) : null}

      {sortedFields.map((field) => {
        const value = values[field.key] ?? field.default;

        if (field.type === "boolean") {
          return (
            <label
              key={field.key}
              className="flex items-start gap-3 rounded-xl border border-glass-border bg-glass/30 px-3 py-3"
            >
              <Checkbox
                checked={Boolean(value)}
                onCheckedChange={(checked) => updateValue(field.key, checked === true)}
              />
              <div className="space-y-1">
                <div className="text-sm font-medium text-text-primary">{field.label}</div>
                {field.description ? (
                  <div className="text-xs text-text-muted">{field.description}</div>
                ) : null}
              </div>
            </label>
          );
        }

        return (
          <label key={field.key} className="block space-y-2">
            <div className="text-sm font-medium text-text-primary">{field.label}</div>
            {field.description ? (
              <div className="text-xs text-text-muted">{field.description}</div>
            ) : null}
            <Input
              type={field.type === "integer" ? "number" : "text"}
              value={String(value ?? "")}
              min={field.min ?? undefined}
              max={field.max ?? undefined}
              onChange={(event) => {
                const nextValue =
                  field.type === "integer"
                    ? Number.parseInt(event.target.value, 10) || 0
                    : event.target.value;
                updateValue(field.key, nextValue);
              }}
            />
          </label>
        );
      })}

      {hasFields ? (
        <div className="flex justify-end">
          <Button size="sm" onClick={() => void save()} disabled={isSaving}>
            {isSaving ? "Saving..." : "Save settings"}
          </Button>
        </div>
      ) : null}
    </div>
  );
}
