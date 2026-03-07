import { Puzzle, Wrench, LayoutPanelTop, RefreshCcw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import type { PluginPanel, PluginSummary } from "@/types/plugin";

interface PluginListProps {
  plugins: PluginSummary[];
  panels: PluginPanel[];
  isLoading: boolean;
  error: string | null;
  onRefresh: () => void;
  onOpenPanel: (label: string) => void;
}

export function PluginList({
  plugins,
  panels,
  isLoading,
  error,
  onRefresh,
  onOpenPanel,
}: PluginListProps) {
  if (isLoading && plugins.length === 0) {
    return <div className="text-sm text-text-muted">Loading plugins...</div>;
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between rounded-2xl border border-glass-border bg-glass/60 px-4 py-3">
        <div>
          <p className="text-xs uppercase tracking-[0.2em] text-text-muted">Plugin System</p>
          <h2 className="mt-1 text-base font-semibold text-text-primary">
            Local WASM plugins
          </h2>
        </div>
        <Button size="sm" variant="ghost" onClick={onRefresh}>
          <RefreshCcw size={14} />
          Refresh
        </Button>
      </div>

      {error ? (
        <div className="rounded-xl border border-danger/30 bg-danger/10 px-3 py-2 text-sm text-danger">
          {error}
        </div>
      ) : null}

      {plugins.length === 0 ? (
        <div className="rounded-2xl border border-dashed border-glass-border bg-glass/30 px-4 py-6 text-sm text-text-secondary">
          No plugins found yet. Build one with `just plugin-build example-minimal` or install one
          into `~/.peekoo/plugins/`.
        </div>
      ) : (
        <div className="space-y-3">
          {plugins.map((plugin) => {
            const pluginPanels = panels.filter((panel) => panel.pluginKey === plugin.pluginKey);

            return (
              <section
                key={plugin.pluginKey}
                className="rounded-2xl border border-glass-border bg-glass/50 p-4 shadow-panel"
              >
                <div className="flex items-start justify-between gap-3">
                  <div className="flex items-start gap-3">
                    <div className="flex h-10 w-10 items-center justify-center rounded-2xl bg-space-overlay text-glow-cyan">
                      <Puzzle size={18} />
                    </div>
                    <div>
                      <div className="flex items-center gap-2">
                        <h3 className="text-sm font-semibold text-text-primary">{plugin.name}</h3>
                        <Badge variant={plugin.enabled ? "default" : "outline"}>
                          {plugin.enabled ? "Loaded" : "Discovered"}
                        </Badge>
                      </div>
                      <p className="mt-1 text-xs text-text-muted">
                        {plugin.pluginKey} · v{plugin.version}
                        {plugin.author ? ` · ${plugin.author}` : ""}
                      </p>
                    </div>
                </div>
              </div>

                {plugin.description ? (
                  <p className="mt-3 text-sm leading-6 text-text-secondary">{plugin.description}</p>
                ) : null}

                <div className="mt-4 grid grid-cols-2 gap-3 text-xs text-text-muted md:grid-cols-3">
                  <div className="rounded-xl border border-glass-border bg-space-deep/50 px-3 py-2">
                    <div className="flex items-center gap-2 text-text-secondary">
                      <Wrench size={12} /> Tools
                    </div>
                    <div className="mt-1 text-sm font-medium text-text-primary">{plugin.toolCount}</div>
                  </div>
                  <div className="rounded-xl border border-glass-border bg-space-deep/50 px-3 py-2">
                    <div className="flex items-center gap-2 text-text-secondary">
                      <LayoutPanelTop size={12} /> Panels
                    </div>
                    <div className="mt-1 text-sm font-medium text-text-primary">{plugin.panelCount}</div>
                  </div>
                  <div className="rounded-xl border border-glass-border bg-space-deep/50 px-3 py-2 col-span-2 md:col-span-1">
                    <div className="text-text-secondary">Location</div>
                    <div className="mt-1 truncate text-sm font-medium text-text-primary">
                      {plugin.pluginDir}
                    </div>
                  </div>
                </div>

                {pluginPanels.length > 0 ? (
                  <div className="mt-4 space-y-2">
                    <p className="text-xs uppercase tracking-[0.16em] text-text-muted">
                      Open panels
                    </p>
                    <div className="flex flex-wrap gap-2">
                      {pluginPanels.map((panel) => (
                        <Button
                          key={panel.label}
                          size="sm"
                          variant="outline"
                          className="bg-space-deep/50"
                          onClick={() => onOpenPanel(panel.label)}
                        >
                          {panel.title}
                        </Button>
                      ))}
                    </div>
                  </div>
                ) : null}
              </section>
            );
          })}
        </div>
      )}
    </div>
  );
}
