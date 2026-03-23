import { Play, Pause, RotateCcw } from "lucide-react";
import { Button } from "@/components/ui/button";

interface TimerControlsProps {
  isActive: boolean;
  onToggle: () => void;
  onReset: () => void;
  onSwitchMode: () => void;
  mode: "work" | "break";
}

export function TimerControls({
  isActive,
  onToggle,
  onReset,
  onSwitchMode,
  mode,
}: TimerControlsProps) {
  return (
    <div className="flex flex-col items-center gap-3 w-full">
      <div className="flex gap-2">
        <Button
          onClick={onToggle}
          variant={isActive ? "warning" : "success"}
          size="sm"
          className="h-9 px-4"
        >
          {isActive ? (
            <>
              <Pause size={16} className="mr-2" /> Pause
            </>
          ) : (
            <>
              <Play size={16} className="mr-2" /> Start
            </>
          )}
        </Button>
        <Button
          onClick={onReset}
          variant="outline"
          size="sm"
          className="h-9 px-4"
        >
          <RotateCcw size={16} className="mr-2" /> Reset
        </Button>
      </div>

      <Button
        onClick={onSwitchMode}
        variant="soft"
        size="sm"
        className="h-8 text-[11px] px-3"
      >
        Switch to {mode === "work" ? "Break" : "Work"}
      </Button>

    </div>
  );
}
