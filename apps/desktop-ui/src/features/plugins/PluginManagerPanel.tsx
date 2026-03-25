import { useState } from "react";
import { openPanelWindow } from "@/hooks/use-panel-windows";
import { usePlugins } from "@/hooks/use-plugins";
import { usePluginStore } from "@/hooks/use-plugin-store";
import { ScrollArea } from "@/components/ui/scroll-area";
import { PluginList } from "./PluginList";
import { PluginStoreCatalog } from "./PluginStoreCatalog";
import { useTranslation } from "react-i18next";

type TabKey = "installed" | "store";

export function PluginManagerPanel() {
  const { t } = useTranslation();
  const { plugins, panels, isLoading, error, refresh, setPluginEnabled, isToggling } = usePlugins();
  const {
    catalog,
    isLoading: isStoreLoading,
    error: storeError,
    fetchCatalog,
    install,
    update,
    uninstall,
    isInstalling,
  } = usePluginStore();
  const [activeTab, setActiveTab] = useState<TabKey>("installed");

  const handleInstall = async (pluginKey: string) => {
    await install(pluginKey);
    await refresh();
  };

  const handleUninstall = async (pluginKey: string) => {
    await uninstall(pluginKey);
    await refresh();
  };

  const handleToggle = async (pluginKey: string, enabled: boolean) => {
    await setPluginEnabled(pluginKey, enabled);
    await refresh();
  };

  const handleUpdate = async (pluginKey: string) => {
    await update(pluginKey);
    await refresh();
    await fetchCatalog();
  };

  return (
    <div className="h-full flex flex-col min-w-0">
      <div className="flex items-center gap-2 px-2 py-2 border-b border-glass-border shrink-0">
        <button
          onClick={() => setActiveTab("installed")}
          className={`px-3 py-1.5 text-sm rounded-lg transition-colors ${
            activeTab === "installed"
              ? "bg-glass text-text-primary"
              : "text-text-muted hover:text-text-secondary"
          }`}
        >
          {t("plugins.tabs.installed")}
        </button>
        <button
          onClick={() => {
            setActiveTab("store");
            void fetchCatalog();
          }}
          className={`px-3 py-1.5 text-sm rounded-lg transition-colors ${
            activeTab === "store"
              ? "bg-glass text-text-primary"
              : "text-text-muted hover:text-text-secondary"
          }`}
        >
          {t("plugins.tabs.store")}
        </button>
      </div>

      <ScrollArea className="flex-1 min-w-0">
        {activeTab === "installed" ? (
          <PluginList
            plugins={plugins}
            panels={panels}
            isLoading={isLoading}
            error={error}
            onRefresh={() => void refresh()}
            onOpenPanel={(label) => {
              void openPanelWindow(label, panels);
            }}
            onToggleEnabled={handleToggle}
            isToggling={isToggling}
            onRemove={handleUninstall}
          />
        ) : (
          <PluginStoreCatalog
            catalog={catalog}
            isLoading={isStoreLoading}
            error={storeError}
            onInstall={handleInstall}
            onUpdate={handleUpdate}
            onUninstall={handleUninstall}
            isInstalling={isInstalling}
            onRefresh={fetchCatalog}
          />
        )}
      </ScrollArea>
    </div>
  );
}
