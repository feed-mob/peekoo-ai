import { Download, LayoutPanelTop, Loader2, Puzzle, RefreshCcw, Trash2, Wrench } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import type { StorePlugin } from "@/types/plugin";
import { useTranslation } from "react-i18next";

function formatPermission(permission: string, t: (key: string) => string): string {
  const key = `plugins.permissionLabels.${permission}`;
  const translated = t(key);
  return translated === key ? permission : translated;
}

interface PluginStoreCatalogProps {
  catalog: StorePlugin[];
  isLoading: boolean;
  error: string | null;
  onInstall: (pluginKey: string) => Promise<void>;
  onUpdate: (pluginKey: string) => Promise<void>;
  onUninstall: (pluginKey: string) => Promise<void>;
  isInstalling: (pluginKey: string) => boolean;
  onRefresh: () => void;
}

export function PluginStoreCatalog({
  catalog,
  isLoading,
  error,
  onInstall,
  onUpdate,
  onUninstall,
  isInstalling,
  onRefresh,
}: PluginStoreCatalogProps) {
  const { t } = useTranslation();
  if (isLoading && catalog.length === 0) {
    return <div className="text-sm text-text-muted">{t("plugins.loadingStore")}</div>;
  }

  return (
    <div className="space-y-4 min-w-0">
      <div className="flex items-center justify-between rounded-2xl border border-glass-border bg-glass/60 px-4 py-3">
        <div className="min-w-0">
          <p className="text-xs uppercase tracking-[0.2em] text-text-muted">{t("plugins.storeLabel")}</p>
          <h2 className="mt-1 text-base font-semibold text-text-primary">
            {t("plugins.availableFromGithub")}
          </h2>
        </div>
        <Button size="sm" variant="ghost" className="shrink-0" onClick={onRefresh}>
          <RefreshCcw size={14} />
          {t("common.refresh")}
        </Button>
      </div>

      {error ? (
        <div className="rounded-xl border border-danger/30 bg-danger/10 px-3 py-2 text-sm text-danger">
          {error}
        </div>
      ) : null}

      {catalog.length === 0 ? (
        <div className="rounded-2xl border border-dashed border-glass-border bg-glass/30 px-4 py-6 text-sm text-text-secondary">
          {t("plugins.emptyStore")}
        </div>
      ) : (
        <div className="space-y-3">
          {catalog.map((plugin) => {
            const installing = isInstalling(plugin.pluginKey);

            return (
              <section
                key={plugin.pluginKey}
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
                        <Badge variant={plugin.installed ? "default" : "outline"}>
                          {plugin.installed ? t("plugins.installed") : t("plugins.available")}
                        </Badge>
                        {plugin.hasUpdate ? <Badge variant="secondary">{t("plugins.updateAvailable")}</Badge> : null}
                      </div>
                      <p className="mt-1 truncate text-xs text-text-muted">
                        {plugin.pluginKey} · v{plugin.version}
                        {plugin.author ? ` · ${plugin.author}` : ""}
                      </p>
                    </div>
                  </div>

                  <div className="flex shrink-0 items-center gap-2">
                    {plugin.installed ? (
                      <>
                        {plugin.hasUpdate ? (
                          <Button
                            size="sm"
                            variant="secondary"
                            className="shrink-0"
                            onClick={() => void onUpdate(plugin.pluginKey)}
                            disabled={installing}
                          >
                            {installing ? (
                              <Loader2 size={14} className="animate-spin" />
                            ) : (
                              <RefreshCcw size={14} />
                            )}
                            {t("plugins.update")}
                          </Button>
                        ) : null}
                        <Button
                          size="icon"
                          variant="outline"
                          className="text-danger border-danger/30 hover:bg-danger/10"
                          title={t("plugins.removePlugin")}
                          onClick={() => void onUninstall(plugin.pluginKey)}
                          disabled={installing}
                        >
                          {installing ? (
                            <Loader2 size={14} className="animate-spin" />
                          ) : (
                            <Trash2 size={16} />
                          )}
                        </Button>
                      </>
                    ) : (
                      <Button
                        size="sm"
                        variant="default"
                        className="shrink-0"
                        onClick={() => void onInstall(plugin.pluginKey)}
                        disabled={installing}
                      >
                        {installing ? (
                          <Loader2 size={14} className="animate-spin" />
                        ) : (
                          <Download size={14} />
                        )}
                        {t("plugins.install")}
                      </Button>
                    )}
                  </div>
                </div>

                {plugin.description ? (
                  <p className="mt-3 text-sm leading-6 text-text-secondary break-words">{plugin.description}</p>
                ) : null}

                {plugin.permissions.length > 0 ? (
                  <div className="mt-4">
                    <p className="text-[11px] uppercase tracking-[0.16em] text-text-muted">{t("plugins.permissions")}</p>
                    <div className="mt-2 flex flex-wrap gap-2">
                      {plugin.permissions.map((permission) => (
                        <Badge
                          key={permission}
                          variant="outline"
                          className="border-glass-border bg-space-deep/40 text-text-secondary"
                          title={permission}
                        >
                          {formatPermission(permission, t)}
                        </Badge>
                      ))}
                    </div>
                  </div>
                ) : null}

                <div className="mt-4 grid grid-cols-2 gap-3 text-xs text-text-muted">
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
                </div>
              </section>
            );
          })}

        </div>
      )}
    </div>
  );
}
