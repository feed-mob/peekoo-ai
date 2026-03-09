import { openPanelWindow } from "@/hooks/use-panel-windows";
import { usePlugins } from "@/hooks/use-plugins";
import { ScrollArea } from "@/components/ui/scroll-area";
import { PluginList } from "./PluginList";

export function PluginManagerPanel() {
  const { plugins, panels, isLoading, error, refresh } = usePlugins();

  return (
    <ScrollArea className="h-full pr-2">
      <PluginList
        plugins={plugins}
        panels={panels}
        isLoading={isLoading}
        error={error}
        onRefresh={() => void refresh()}
        onOpenPanel={(label) => {
          void openPanelWindow(label, panels);
        }}
      />
    </ScrollArea>
  );
}
