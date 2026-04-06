import { Loader2, LayoutPanelTop, Puzzle, RefreshCcw, Trash2, Wrench } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { PluginPanel, PluginSummary } from "@/types/plugin";
import { PluginConfigPanel } from "./PluginConfigPanel";
import { useTranslation } from "react-i18next";

interface PluginListProps {
  plugins: PluginSummary[];
  panels: PluginPanel[];
  isLoading: boolean;
  error: string | null;
  onRefresh: () => void;
  onOpenPanel: (label: string) => void;
  onToggleEnabled: (pluginKey: string, enabled: boolean) => Promise<void>;
  isToggling: (pluginKey: string) => boolean;
  onRemove?: (pluginKey: string) => Promise<void>;
}

export function PluginList({
  plugins,
  panels,
  isLoading,
  error,
  onRefresh,
  onOpenPanel,
  onToggleEnabled,
  isToggling,
  onRemove,
}: PluginListProps) {
  const { t } = useTranslation();
  if (isLoading && plugins.length === 0) {
    return <div className="text-sm text-text-muted">{t("plugins.loadingInstalled")}</div>;
  }

  return (
    <div className="space-y-4 min-w-0">
      <div className="flex items-center justify-between rounded-2xl border border-glass-border bg-glass/60 px-4 py-3">
        <div>
          <p className="text-xs uppercase tracking-[0.2em] text-text-muted">{t("plugins.system")}</p>
          <h2 className="mt-1 text-base font-semibold text-text-primary">
            {t("plugins.installedTitle")}
          </h2>
        </div>
        <Button size="sm" variant="ghost" onClick={onRefresh}>
          <RefreshCcw size={14} />
          {t("common.refresh")}
        </Button>
      </div>

      {error ? (
        <div className="rounded-xl border border-danger/30 bg-danger/10 px-3 py-2 text-sm text-danger">
          {error}
        </div>
      ) : null}

      {plugins.length === 0 ? (
        <div className="rounded-2xl border border-dashed border-glass-border bg-glass/30 px-4 py-6 text-sm text-text-secondary">
          {t("plugins.emptyInstalled")}
        </div>
      ) : (
        <div className="space-y-3">
          {plugins.map((plugin) => {
            const pluginPanels = panels.filter((panel) => panel.pluginKey === plugin.pluginKey);
            const toggling = isToggling(plugin.pluginKey);

            return (
              <section
                key={plugin.pluginDir}
                className="rounded-2xl border border-glass-border bg-glass/50 p-4 shadow-panel overflow-hidden"
              >
                <div className="flex items-start justify-between gap-3">
                  <div className="flex min-w-0 items-start gap-3">
                    <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-2xl bg-space-overlay text-glow-cyan">
                      <Puzzle size={18} />
                    </div>
                    <div className="min-w-0">
                      <div className="flex items-center gap-2">
                        <h3 className="truncate text-sm font-semibold text-text-primary">{plugin.name}</h3>
                        <Badge variant={plugin.enabled ? "default" : "outline"}>
                          {plugin.enabled ? t("plugins.enabled") : t("plugins.disabled")}
                        </Badge>
                      </div>
                      <p className="mt-1 truncate text-xs text-text-muted">
                        {plugin.pluginKey} · v{plugin.version}
                        {plugin.author ? ` · ${plugin.author}` : ""}
                      </p>
                    </div>
                  </div>

                  <div className="flex shrink-0 items-center gap-2">
                    <Button
                      size="sm"
                      variant={plugin.enabled ? "outline" : "default"}
                      className={plugin.enabled ? "bg-space-deep/50" : undefined}
                      onClick={() => void onToggleEnabled(plugin.pluginKey, !plugin.enabled)}
                      disabled={toggling}
                    >
                      {toggling ? <Loader2 size={14} className="animate-spin" /> : null}
                      {plugin.enabled ? t("plugins.disable") : t("plugins.enable")}
                    </Button>
                    {onRemove ? (
                      <Button
                        size="icon"
                        variant="outline"
                        className="text-danger border-danger/30 hover:bg-danger/10 shrink-0"
                        title={t("plugins.removePlugin")}
                        onClick={() => void onRemove(plugin.pluginKey)}
                        disabled={toggling}
                      >
                        <Trash2 size={16} />
                      </Button>
                    ) : null}
                  </div>
                </div>

                {plugin.description ? (
                  <p className="mt-3 text-sm leading-6 text-text-secondary break-words">{plugin.description}</p>
                ) : null}

                <div className="mt-4 grid grid-cols-2 gap-3 text-xs text-text-muted md:grid-cols-3">
                  <div className="rounded-xl border border-glass-border bg-space-deep/50 px-3 py-2">
                    <div className="flex items-center gap-2 text-text-secondary">
                      <Wrench size={12} /> {t("plugins.tools")}
                    </div>
                    <div className="mt-1 text-sm font-medium text-text-primary">{plugin.toolCount}</div>
                  </div>
                  <div className="rounded-xl border border-glass-border bg-space-deep/50 px-3 py-2">
                    <div className="flex items-center gap-2 text-text-secondary">
                      <LayoutPanelTop size={12} /> {t("plugins.panels")}
                    </div>
                    <div className="mt-1 text-sm font-medium text-text-primary">{plugin.panelCount}</div>
                  </div>
                  <div className="rounded-xl border border-glass-border bg-space-deep/50 px-3 py-2 col-span-2 md:col-span-1 min-w-0">
                     <div className="text-text-secondary">{t("plugins.location")}</div>
                     <div className="mt-1 text-sm font-medium text-text-primary break-all">
                       {plugin.pluginDir}
                     </div>
                   </div>
                </div>

                {pluginPanels.length > 0 ? (
                  <div className="mt-4 space-y-2">
                    <p className="text-xs uppercase tracking-[0.16em] text-text-muted">
                      {t("plugins.openPanels")}
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
                {!plugin.enabled ? (
                  <p className="mt-4 text-xs text-text-muted">
                    {t("plugins.enableHint")}
                  </p>
                ) : null}

                <PluginConfigPanel pluginKey={plugin.pluginKey} />
              </section>
            );
          })}
        </div>
      )}
    </div>
  );
}
