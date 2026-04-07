import { Play, Pause, RotateCcw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useTranslation } from "react-i18next";

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
  const { t } = useTranslation();
  return (
    <div className="flex flex-col items-center gap-3 w-full shrink-0">
      <div className="flex gap-2 shrink-0">
        <Button
          onClick={onToggle}
          variant={isActive ? "warning" : "success"}
          size="sm"
          className="h-9 px-4"
        >
          {isActive ? (
            <>
              <Pause size={16} className="mr-2" /> {t("pomodoro.controls.pause")}
            </>
          ) : (
            <>
              <Play size={16} className="mr-2" /> {t("pomodoro.controls.start")}
            </>
          )}
        </Button>
        <Button
          onClick={onReset}
          variant="outline"
          size="sm"
          className="h-9 px-4"
        >
          <RotateCcw size={16} className="mr-2" /> {t("pomodoro.controls.reset")}
        </Button>
      </div>

      <Button
        onClick={onSwitchMode}
        variant="soft"
        size="sm"
        className="h-8 text-[11px] px-3"
      >
        {mode === "work"
          ? t("pomodoro.controls.switchToBreak")
          : t("pomodoro.controls.switchToWork")}
      </Button>

    </div>
  );
}
