import { lazy, Suspense } from "react";
import { WindowLabelSchema } from "@/types/window";

const SpriteView = lazy(() => import("@/views/SpriteView"));
const ChatView = lazy(() => import("@/views/ChatView"));
const TasksView = lazy(() => import("@/views/TasksView"));
const PomodoroView = lazy(() => import("@/views/PomodoroView"));

function UnknownView({ label }: { label: string }) {
  return (
    <div className="flex items-center justify-center w-full h-screen text-text-muted">
      Unknown window: {label}
    </div>
  );
}

function viewForLabel(label: string) {
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
  }
}

export function ResolvedView({ label }: { label: string }) {
  return <Suspense fallback={null}>{viewForLabel(label)}</Suspense>;
}
