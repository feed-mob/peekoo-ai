import { Play, Pause, RotateCcw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";

interface TimerControlsProps {
  isActive: boolean;
  onToggle: () => void;
  onReset: () => void;
  onSwitchMode: () => void;
  mode: "work" | "break";
  completedSessions: number;
}

export function TimerControls({
  isActive,
  onToggle,
  onReset,
  onSwitchMode,
  mode,
  completedSessions,
}: TimerControlsProps) {
  return (
    <div className="flex flex-col items-center gap-4 w-full">
      <div className="flex gap-3">
        <Button
          onClick={onToggle}
          variant={isActive ? "warning" : "success"}
        >
          {isActive ? (
            <>
              <Pause size={18} className="mr-2" /> Pause
            </>
          ) : (
            <>
              <Play size={18} className="mr-2" /> Start
            </>
          )}
        </Button>
        <Button
          onClick={onReset}
          variant="outline"
        >
          <RotateCcw size={18} className="mr-2" /> Reset
        </Button>
      </div>

      <Button
        onClick={onSwitchMode}
        variant="soft"
        size="sm"
      >
        Switch to {mode === "work" ? "Break" : "Work"}
      </Button>

      <Badge
        variant="outline"
        className="border-accent-teal/30 text-accent-teal"
      >
        {completedSessions} sessions completed
      </Badge>
    </div>
  );
}
