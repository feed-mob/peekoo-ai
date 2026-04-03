import { PanelShell } from "@/components/panels/PanelShell";
import { PluginManagerPanel } from "@/features/plugins/PluginManagerPanel";
import { useTranslation } from "react-i18next";

export default function PluginsView() {
  const { t } = useTranslation();
  return (
    <PanelShell title={t("panel.plugins")}>
      <PluginManagerPanel />
    </PanelShell>
  );
}
