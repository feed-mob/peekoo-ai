import { PanelShell } from "@/components/panels/PanelShell";
import { TasksPanel } from "@/features/tasks/TasksPanel";
import { useTranslation } from "react-i18next";

export default function TasksView() {
  const { t } = useTranslation();
  return (
    <PanelShell title={t("panel.tasks")}>
      <TasksPanel />
    </PanelShell>
  );
}
