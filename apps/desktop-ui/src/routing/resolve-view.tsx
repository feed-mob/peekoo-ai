import { lazy, Suspense } from "react";
import { WindowLabelSchema, BUILTIN_PANEL_LABELS } from "@/types/window";

const SpriteView = lazy(() => import("@/views/SpriteView"));
const ChatView = lazy(() => import("@/views/ChatView"));
const TasksView = lazy(() => import("@/views/TasksView"));
const PomodoroView = lazy(() => import("@/views/PomodoroView"));
const PluginsView = lazy(() => import("@/views/PluginsView"));
const PluginPanelView = lazy(() => import("@/views/PluginPanelView"));

function UnknownView({ label }: { label: string }) {
  return (
    <div className="flex items-center justify-center w-full h-screen text-text-muted">
      Unknown window: {label}
    </div>
  );
}

function viewForLabel(label: string) {
  if (
    label.startsWith("panel-") &&
    !(BUILTIN_PANEL_LABELS as readonly string[]).includes(label)
  ) {
    return <PluginPanelView />;
  }

  const parsed = WindowLabelSchema.safeParse(label);
  if (!parsed.success) return <UnknownView label={label} />;

  switch (parsed.data) {
    case "main":
      return <SpriteView />;
    case "panel-chat":
      return <ChatView />;
    case "panel-tasks":
      return <TasksView />;
    case "panel-pomodoro":
      return <PomodoroView />;
    case "panel-plugins":
      return <PluginsView />;
  }
}

export function ResolvedView({ label }: { label: string }) {
  return <Suspense fallback={null}>{viewForLabel(label)}</Suspense>;
}
