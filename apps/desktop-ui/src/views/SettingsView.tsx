import { PanelShell } from "@/components/panels/PanelShell";
import { SettingsPanel } from "@/features/settings/SettingsPanel";
import { useTranslation } from "react-i18next";

export default function SettingsView() {
  const { t } = useTranslation();
  return (
    <PanelShell title={t("panel.settings")}>
      <SettingsPanel />
    </PanelShell>
  );
}
