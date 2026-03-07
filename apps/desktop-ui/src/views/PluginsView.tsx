import { PanelShell } from "@/components/panels/PanelShell";
import { PluginManagerPanel } from "@/features/plugins/PluginManagerPanel";

export default function PluginsView() {
  return (
    <PanelShell title="Plugins">
      <PluginManagerPanel />
    </PanelShell>
  );
}
