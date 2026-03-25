import { PanelShell } from "@/components/panels/PanelShell";
import { PomodoroPanel } from "@/features/pomodoro/PomodoroPanel";
import { useTranslation } from "react-i18next";

export default function PomodoroView() {
  const { t } = useTranslation();
  return (
    <PanelShell title={t("panel.pomodoro")}>
      <PomodoroPanel />
    </PanelShell>
  );
}
