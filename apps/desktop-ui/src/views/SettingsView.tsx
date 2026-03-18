import { PanelShell } from "@/components/panels/PanelShell";
import { SettingsPanel } from "@/features/settings/SettingsPanel";

export default function SettingsView() {
  return (
    <PanelShell title="Settings">
      <SettingsPanel />
    </PanelShell>
  );
}
