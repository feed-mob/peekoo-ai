import { PanelShell } from "@/components/panels/PanelShell";
import { TasksPanel } from "@/features/tasks/TasksPanel";

export default function TasksView() {
  return (
    <PanelShell title="Tasks">
      <TasksPanel />
    </PanelShell>
  );
}
