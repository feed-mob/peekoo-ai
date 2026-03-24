import { AnimatePresence, motion } from "framer-motion";
import { Droplet, Eye, PersonStanding, Brain, Coffee, type LucideIcon } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { finishPomodoro } from "@/features/pomodoro/tool-client";
import { BUBBLE_EXTRA_HEIGHT, BUBBLE_WIDTH } from "@/lib/sprite-bubble-layout";
import { useIsDarkMode } from "@/hooks/use-is-dark-mode";
import { cn } from "@/lib/utils";
import type { SpriteBubblePayload } from "@/types/sprite-bubble";

interface SpriteBubbleProps {
  payload: SpriteBubblePayload | null;
  visible: boolean;
}

const ICON_MAP: Record<string, LucideIcon> = {
  water: Droplet,
  eye: Eye,
  stand: PersonStanding,
  focus: Brain,
  break: Coffee,
};

const COLOR_MAP: Record<string, string> = {
  water: "#3b82f6",
  eye: "#22c55e",
  stand: "#eab308",
  focus: "#689B8A",
  break: "#E9762B",
};


export function SpriteBubble({ payload, visible }: SpriteBubbleProps) {
  const isDark = useIsDarkMode();
  const bodyLower = payload?.body.toLowerCase() || "";
  const titleLower = payload?.title.toLowerCase() || "";
  
  const isHealth = titleLower.includes("health");
  const isPomodoroWork = titleLower.includes("focus");
  const isPomodoroBreak = titleLower.includes("break");
  
  const type = isHealth
    ? (bodyLower.includes("water") ? "water" : bodyLower.includes("eye") ? "eye" : bodyLower.includes("stand") ? "stand" : null)
    : isPomodoroWork ? "focus" : isPomodoroBreak ? "break" : null;
    
  const isStyled = isHealth || isPomodoroWork || isPomodoroBreak;

  const Icon = type ? ICON_MAP[type] : null;
  const themeColor = type ? COLOR_MAP[type] : "currentColor";

  const handleDismiss = async () => {
    if (!isStyled || !type) return;
    try {
      if (isHealth) {
        await invoke("plugin_call_tool", {
          toolName: "health_dismiss",
          argsJson: JSON.stringify({ reminder_type: type === "stand" ? "standup" : type === "eye" ? "eye_rest" : "water" })
        });
      } else {
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
              <p className="text-[12px] font-medium leading-tight text-text-primary/90 dark:text-white/90">
                {payload.body}
              </p>
            </div>
          ) : (
            <>
              <p className="text-[9px] font-semibold uppercase tracking-[0.2em] text-glow-cyan/80">
                {payload.title}
              </p>
              <p className="mt-1 text-[13px] leading-[18px] text-text-primary line-clamp-3">
                {payload.body}
              </p>
            </>
          )}
        </motion.div>
      ) : null}
    </AnimatePresence>
  );
}
