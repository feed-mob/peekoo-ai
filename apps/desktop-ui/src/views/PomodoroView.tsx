import { PanelShell } from "@/components/panels/PanelShell";
import { PomodoroPanel } from "@/features/pomodoro/PomodoroPanel";

export default function PomodoroView() {
  return (
    <PanelShell title="Pomodoro">
      <PomodoroPanel />
    </PanelShell>
  );
}
