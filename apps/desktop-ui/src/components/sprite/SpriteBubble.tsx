import { AnimatePresence, motion } from "framer-motion";
import {
  Droplet,
  Eye,
  PersonStanding,
  Brain,
  Coffee,
  CalendarClock,
  ListTodo,
  type LucideIcon,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import type { MouseEvent } from "react";
import { finishPomodoro } from "@/features/pomodoro/tool-client";
import { BUBBLE_EXTRA_HEIGHT, BUBBLE_WIDTH } from "@/lib/sprite-bubble-layout";
import { getSpriteBubbleKind } from "@/lib/sprite-notification-presentation";
import { useIsDarkMode } from "@/hooks/use-is-dark-mode";
import { cn } from "@/lib/utils";
import type { SpriteBubblePayload } from "@/types/sprite-bubble";

interface SpriteBubbleProps {
  payload: SpriteBubblePayload | null;
  visible: boolean;
  onOpenPanel?: (panelLabel: string) => void;
}

const ICON_MAP: Record<string, LucideIcon> = {
  water: Droplet,
  eye: Eye,
  stand: PersonStanding,
  focus: Brain,
  break: Coffee,
  task: ListTodo,
  calendar: CalendarClock,
};

const COLOR_MAP: Record<string, string> = {
  water: "#3b82f6",
  eye: "#22c55e",
  stand: "#eab308",
  focus: "#689B8A",
  break: "#E9762B",
  task: "#22c55e",
  calendar: "#38bdf8",
};


export function SpriteBubble({ payload, visible, onOpenPanel }: SpriteBubbleProps) {
  const isDark = useIsDarkMode();
  const kind = payload ? getSpriteBubbleKind(payload) : "default";
  const type = kind === "default" ? null : kind;
  const isStyled = type !== null;
  const showStyledTitle = kind === "task" || kind === "calendar";
  const isHealth = kind === "water" || kind === "eye" || kind === "stand";

  const Icon = type ? ICON_MAP[type] : null;
  const themeColor = type ? COLOR_MAP[type] : "currentColor";

  const handleActionClick = async (event: MouseEvent) => {
    event.stopPropagation();
    if (!payload?.actionUrl) {
      return;
    }
    try {
      await invoke("system_open_url", { url: payload.actionUrl });
    } catch (err) {
      console.error("Failed to open notification action URL:", err);
    }
  };

  const handleDismiss = async () => {
    if (!isStyled || !type) return;
    if (payload?.panelLabel) {
      onOpenPanel?.(payload.panelLabel);
      return;
    }
    try {
      if (isHealth) {
        await invoke("plugin_call_tool", {
          toolName: "health_dismiss",
          argsJson: JSON.stringify({ reminder_type: type === "stand" ? "standup" : type === "eye" ? "eye_rest" : "water" })
        });
      } else if (kind === "focus" || kind === "break") {
        await finishPomodoro();
      }
    } catch (err) {
      console.error("Failed to dismiss reminder from bubble:", err);
    }
  };

  return (
    <AnimatePresence>
      {payload && visible ? (
        <motion.div
          key={`${payload.title}-${payload.body}`}
          initial={{ opacity: 0, y: 10, scale: 0.96 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          exit={{ opacity: 0, y: 5, scale: 0.98 }}
          transition={{ 
            type: "spring",
            damping: 25,
            stiffness: 150
          }}
          onClick={handleDismiss}
          className={cn(
            "absolute z-20 cursor-pointer",
            isStyled 
             ? cn(
                "rounded-xl transition-all duration-300 hover:shadow-lg pointer-events-auto border shadow-xl backdrop-blur-3xl px-3.5 py-2",
                isDark ? "bg-black/80 border-white/10" : "bg-white/80 border-white/20"
               )
             : "pointer-events-none rounded-2xl border border-glass-border bg-glass/95 px-4 py-3 shadow-panel backdrop-blur-xl"
          )}
          style={{
            bottom: "calc(100% - 15px)",
            left: "50%",
            marginLeft: -(BUBBLE_WIDTH / 2),
            width: BUBBLE_WIDTH,
            maxHeight: BUBBLE_EXTRA_HEIGHT - 16,
          }}
        >
          {/* Non-overlapping Tail pointing down (left-aligned) */}
          <div 
            className={cn(
              "absolute bottom-[-5px] left-8 w-[10px] h-[5px] backdrop-blur-3xl",
              isDark ? "bg-black/80" : "bg-white/80"
            )}
            style={{ 
              clipPath: 'polygon(0% 0%, 100% 0%, 50% 100%)',
              // Add borders to the triangle sides using filter shadow to match bubble border
              filter: `drop-shadow(0px 1px 0px ${isDark ? 'rgba(255,255,255,0.1)' : 'rgba(255,255,255,0.2)'})`
            }}
          />

          {isStyled ? (
            <div className="flex items-start gap-2.5">
              {Icon && (
                <div 
                  className="flex items-center justify-center w-5 h-5 rounded-full shrink-0"
                  style={{ background: `${themeColor}15` }}
                >
                   <Icon 
                     size={12} 
                     style={{ 
                       color: themeColor, 
                       fill: (type !== 'stand' && type !== 'focus' && type !== 'break') ? themeColor : 'none',
                       stroke: themeColor,
                       strokeWidth: (type === 'stand' || type === 'focus' || type === 'break') ? '2.5px' : '0px',
                       filter: `drop-shadow(0 0 4px ${themeColor}88)` 
                     }} 
                   />
                </div>
              )}
              <div className="min-w-0">
                {showStyledTitle ? (
                  <p className="text-[10px] font-semibold uppercase tracking-[0.18em] text-text-primary/60 dark:text-white/60">
                    {payload.title}
                  </p>
                ) : null}
                <p className="text-[12px] font-medium leading-tight text-text-primary/90 dark:text-white/90">
                  {payload.body}
                </p>
                {showStyledTitle && payload.actionUrl && payload.actionLabel && (
                  <button
                    type="button"
                    onClick={handleActionClick}
                    className="mt-1.5 rounded-md px-2 py-0.5 text-[10px] font-semibold text-white"
                    style={{ background: themeColor }}
                  >
                    {payload.actionLabel}
                  </button>
                )}
              </div>
            </div>
          ) : (
            <>
              <p className="text-[9px] font-semibold uppercase tracking-[0.2em] text-glow-cyan/80">
                {payload.title}
              </p>
              <p className="mt-1 text-[13px] leading-[18px] text-text-primary line-clamp-3">
                {payload.body}
              </p>
              {payload.actionUrl && payload.actionLabel && (
                <button
                  type="button"
                  onClick={handleActionClick}
                  className="mt-3 rounded-lg bg-glow-green/90 px-3 py-1.5 text-[11px] font-semibold text-space-void hover:bg-glow-green"
                >
                  {payload.actionLabel}
                </button>
              )}
            </>
          )}
        </motion.div>
      ) : null}
    </AnimatePresence>
  );
}
