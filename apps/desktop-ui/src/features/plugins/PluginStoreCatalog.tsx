import { Puzzle, Wrench, LayoutPanelTop, Download, Trash2, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import type { StorePlugin } from "@/types/plugin";

interface PluginStoreCatalogProps {
  catalog: StorePlugin[];
  isLoading: boolean;
  error: string | null;
  onInstall: (pluginKey: string) => Promise<void>;
  onUninstall: (pluginKey: string) => Promise<void>;
  isInstalling: (pluginKey: string) => boolean;
  onRefresh: () => void;
}

export function PluginStoreCatalog({
  catalog,
  isLoading,
  error,
  onInstall,
  onUninstall,
  isInstalling,
  onRefresh,
}: PluginStoreCatalogProps) {
  if (isLoading && catalog.length === 0) {
    return <div className="text-sm text-text-muted">Loading plugin store...</div>;
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between rounded-2xl border border-glass-border bg-glass/60 px-4 py-3">
        <div>
          <p className="text-xs uppercase tracking-[0.2em] text-text-muted">Plugin Store</p>
          <h2 className="mt-1 text-base font-semibold text-text-primary">
            Available from GitHub
          </h2>
        </div>
        <Button size="sm" variant="ghost" onClick={onRefresh}>
          Refresh
        </Button>
      </div>

      {error ? (
        <div className="rounded-xl border border-danger/30 bg-danger/10 px-3 py-2 text-sm text-danger">
          {error}
        </div>
      ) : null}

      {catalog.length === 0 ? (
        <div className="rounded-2xl border border-dashed border-glass-border bg-glass/30 px-4 py-6 text-sm text-text-secondary">
          No plugins available in the store.
        </div>
      ) : (
        <div className="space-y-3">
          {catalog.map((plugin) => {
            const installing = isInstalling(plugin.pluginKey);

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
                        <Badge variant={plugin.installed ? "default" : "outline"}>
                          {plugin.installed ? "Installed" : "Available"}
                        </Badge>
                      </div>
                      <p className="mt-1 text-xs text-text-muted">
                        {plugin.pluginKey} · v{plugin.version}
                        {plugin.author ? ` · ${plugin.author}` : ""}
                      </p>
                    </div>
                  </div>

                  <div className="flex items-center gap-2">
                    {plugin.installed ? (
                      <Button
                        size="sm"
                        variant="outline"
                        className="text-danger border-danger/30 hover:bg-danger/10"
                        onClick={() => void onUninstall(plugin.pluginKey)}
                        disabled={installing}
                      >
                        {installing ? (
                          <Loader2 size={14} className="animate-spin" />
                        ) : (
                          <Trash2 size={14} />
                        )}
                        Remove
                      </Button>
                    ) : (
                      <Button
                        size="sm"
                        variant="default"
                        onClick={() => void onInstall(plugin.pluginKey)}
                        disabled={installing}
                      >
                        {installing ? (
                          <Loader2 size={14} className="animate-spin" />
                        ) : (
                          <Download size={14} />
                        )}
                        Install
                      </Button>
                    )}
                  </div>
                </div>

                {plugin.description ? (
                  <p className="mt-3 text-sm leading-6 text-text-secondary">{plugin.description}</p>
                ) : null}

                <div className="mt-4 grid grid-cols-2 gap-3 text-xs text-text-muted">
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
                </div>
              </section>
            );
          })}
        </div>
      )}
    </div>
  );
}
