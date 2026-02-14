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
          className={isActive 
            ? "bg-warning hover:bg-warning/80 text-space-void" 
            : "bg-success hover:bg-success/80 text-space-void"
          }
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
          className="border-glass-border text-text-primary hover:bg-space-overlay"
        >
          <RotateCcw size={18} className="mr-2" /> Reset
        </Button>
      </div>

      <Button
        onClick={onSwitchMode}
        variant="ghost"
        size="sm"
        className="text-text-muted hover:text-text-primary"
      >
        Switch to {mode === "work" ? "Break" : "Work"}
      </Button>

      <Badge
        variant="outline"
        className="border-glow-blue/30 text-glow-blue"
      >
        {completedSessions} sessions completed
      </Badge>
    </div>
  );
}
