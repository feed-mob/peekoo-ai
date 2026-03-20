import { AnimatePresence, motion } from "framer-motion";
import { Droplet, Eye, PersonStanding, Activity, Brain, Coffee, Play, Pause, type LucideIcon } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import {
  PEEK_BADGE_EXPANDED_VERTICAL_PADDING,
  PEEK_BADGE_HEIGHT,
  PEEK_BADGE_PADDING,
  PEEK_BADGE_ROW_HEIGHT,
} from "@/lib/sprite-bubble-layout";
import type { PeekBadgeItem } from "@/types/peek-badge";

const BADGE_WIDTH = 188;
const POMODORO_BADGE_WIDTH = 125;
const VALUE_COLUMN_WIDTH = 64;

const ICON_MAP: Record<string, LucideIcon> = {
  droplet: Droplet,
  eye: Eye,
  "person-standing": PersonStanding,
  activity: Activity,
  brain: Brain,
  coffee: Coffee,
};

function BadgeIcon({ name, className }: { name?: string; className?: string }) {
  const Icon = name ? ICON_MAP[name] : undefined;
  if (!Icon) return null;
  return <Icon size={12} className={className} />;
}

interface BadgeRowProps {
  item: PeekBadgeItem;
  compact?: boolean;
}

function BadgeRow({ item, compact }: BadgeRowProps) {
  const rowHeight = compact ? PEEK_BADGE_ROW_HEIGHT : PEEK_BADGE_HEIGHT;

  if (!compact) {
    return (
      <div
        className="flex flex-col justify-center gap-0.5"
        style={{ height: rowHeight }}
      >
        <div className="flex min-w-0 items-center gap-1.5 overflow-hidden">
          <BadgeIcon name={item.icon} className="shrink-0 text-glow-cyan/70" />
          <span className="truncate whitespace-nowrap text-[11px] font-medium leading-none text-text-primary/90">
            {item.label}
          </span>
        </div>
        <span className="truncate pl-[18px] text-[11px] leading-none tabular-nums text-glow-cyan/80">
          {item.value}
        </span>
      </div>
    );
  }

  return (
    <div
      className="grid items-center gap-x-2"
      style={{
        height: rowHeight,
        gridTemplateColumns: `minmax(0, 1fr) ${VALUE_COLUMN_WIDTH}px`,
      }}
    >
      <div className="flex min-w-0 items-center gap-1.5 overflow-hidden">
        <BadgeIcon name={item.icon} className="shrink-0 text-glow-cyan/70" />
        <span className="truncate whitespace-nowrap text-[11px] font-medium leading-none text-text-primary/90">
          {item.label}
        </span>
      </div>
      <span className="truncate text-right text-[11px] leading-none tabular-nums text-glow-cyan/80">
        {item.value}
      </span>
    </div>
  );
}

interface StyledBadgeProps {
  item: PeekBadgeItem;
}

function StyledBadge({ item }: StyledBadgeProps) {
  const isPomodoro = item.icon === "brain" || item.icon === "coffee";
  const isPaused = item.label.includes("(Paused)");
  
  const lightGray = "#75787B";
  const darkWhite = "rgba(255, 255, 255, 0.9)";
  
  const handleControl = async (e: React.MouseEvent) => {
    e.stopPropagation();
    
    if (isPomodoro) {
        const toolName = isPaused ? "pomodoro_resume" : "pomodoro_pause";
        try {
            await invoke("plugin_call_tool", {
              toolName,
              argsJson: JSON.stringify({})
            });
        } catch (err) {
            console.error("Failed to toggle pomodoro from badge:", err);
        }
    } else {
        // Health reminder - dismiss/reset timer
        let reminder_type = item.icon === "droplet" ? "water" : item.icon === "eye" ? "eye_rest" : "standup";
        try {
            await invoke("plugin_call_tool", {
                toolName: "health_dismiss",
                argsJson: JSON.stringify({ reminder_type })
            });
        } catch (err) {
            console.error("Failed to dismiss health reminder from badge:", err);
        }
    }
  };

  const Icon = item.icon ? ICON_MAP[item.icon] : undefined;

  const healthColors: Record<string, string> = {
    droplet: "#3b82f6", // Blue
    "person-standing": "#eab308", // Yellow
    eye: "#22c55e", // Green
  };
  const iconColor = (item.icon && healthColors[item.icon]) || (document.documentElement.classList.contains('dark') ? darkWhite : lightGray);
  const isReminder = !isPomodoro;

  const healthBgs: Record<string, string> = {
    droplet: "bg-blue-500/10 dark:bg-blue-400/10 border-blue-500/20 dark:border-blue-400/20",
    "person-standing": "bg-yellow-500/10 dark:bg-yellow-400/10 border-yellow-500/20 dark:border-yellow-400/20",
    eye: "bg-green-500/10 dark:bg-green-400/10 border-green-500/20 dark:border-green-400/20",
  };
  
  const containerStyle = isPomodoro 
    ? "bg-white/60 dark:bg-black/60 border-white/10 dark:border-white/5" 
    : (item.icon && healthBgs[item.icon]) ? healthBgs[item.icon] : "bg-white/10 dark:bg-black/20 border-white/20 dark:border-white/10";

  return (
    <motion.div 
      className={`flex items-center justify-between w-full h-full pl-3.5 pr-2.5 rounded-full backdrop-blur-3xl shadow-panel/10 border transition-all duration-300 ${containerStyle}`}
      animate={isReminder ? {
        scale: [1, 1.02, 1],
        boxShadow: [
          "0 0 0px rgba(0,0,0,0)",
          `0 0 12px ${iconColor}44`,
          "0 0 0px rgba(0,0,0,0)"
        ]
      } : {}}
      transition={isReminder ? {
        duration: 2,
        repeat: Infinity,
        ease: "easeInOut"
      } : {}}
    >
      <div className="flex flex-col items-center justify-center min-w-0 flex-1">
         <span 
           className="text-[22px] font-[100] tabular-nums tracking-[0.05em] whitespace-nowrap leading-none select-none"
           style={{ 
             color: 'transparent',
             fontFamily: '-apple-system, "SF Pro Display", "Helvetica Neue", Arial, sans-serif',
           }}
         >
           <span className="block dark:hidden" style={{ WebkitTextStroke: `0.7px ${lightGray}` }}>
              {item.value}
           </span>
           <span className="hidden dark:block" style={{ WebkitTextStroke: `0.7px ${darkWhite}` }}>
              {item.value}
           </span>
         </span>
      </div>

      <div className="flex items-center justify-center ml-2">
          <button
            onClick={handleControl}
            className={`w-[22px] h-[22px] flex-shrink-0 flex items-center justify-center rounded-[4px] backdrop-blur-3xl hover:bg-current/10 active:scale-95 transition-all group overflow-hidden relative border-[0.6px] ${isPomodoro ? 'border-dashed border-black/20 dark:border-white/20' : 'border-none'}`}
            style={{ 
                background: 'rgba(255, 255, 255, 0.05)',
            }}
          >
            <div className={`w-full h-full flex items-center justify-center transition-opacity`}>
              {isPomodoro ? (
                isPaused ? (
                    <Play 
                      className="w-[12px] h-[12px] mb-[-0.5px]" 
                      style={{ 
                          stroke: 'currentColor',
                          strokeWidth: '1.5px',
                          fill: 'none',
                          color: window.matchMedia("(prefers-color-scheme: dark)").matches ? darkWhite : lightGray
                      }} 
                    />
                ) : (
                    <Pause 
                      className="w-[11px] h-[11px]" 
                      style={{ 
                          color: 'transparent',
                          stroke: document.documentElement.classList.contains('dark') ? darkWhite : lightGray,
                          strokeWidth: '1.2px',
                          fill: 'none',
                      }} 
                    />
                )
              ) : (
                Icon && (
                  <Icon 
                    className="w-[12px] h-[12px]" 
                    style={{ 
                        color: iconColor,
                        fill: (item.icon === "droplet" || item.icon === "eye") ? iconColor : 'none',
                        stroke: (item.icon === "person-standing") ? iconColor : 'transparent',
                        strokeWidth: '1.5px',
                        filter: `drop-shadow(0 0 3px ${iconColor}88)`
                    }} 
                  />
                )
              )}
              
              <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
                 {isPomodoro ? (
                    isPaused ? (
                       <Play className={`w-[12px] h-[12px] mb-[-0.5px] text-[#75787B] dark:text-white/90 fill-none stroke-[1.2px] stroke-current`} />
                    ) : (
                       <Pause className={`w-[11px] h-[11px] text-[#75787B] dark:text-white/90 fill-none stroke-[1.2px] stroke-current`} />
                    )
                 ) : (
                    Icon && (
                      <Icon 
                        className={`w-[11px] h-[11px]`} 
                        style={{
                           color: iconColor,
                           fill: (item.icon === "droplet" || item.icon === "eye") ? iconColor : 'none',
                           stroke: (item.icon === "person-standing") ? iconColor : 'none',
                           strokeWidth: (item.icon === "person-standing") ? '1.5px' : '0',
                           filter: `drop-shadow(0 0 2px ${iconColor}aa)`
                        }}
                      />
                    )
                 )}
              </div>
            </div>
          </button>
      </div>
    </motion.div>
  );
}

interface SpritePeekBadgeProps {
  items: PeekBadgeItem[];
  currentItem: PeekBadgeItem | null;
  expanded: boolean;
  visible: boolean;
  onToggle: () => void;
}

export function SpritePeekBadge({
  items,
  currentItem,
  expanded,
  visible,
  onToggle,
}: SpritePeekBadgeProps) {
  if (!visible || items.length === 0 || !currentItem) return null;

  const isStyledBadge = ["brain", "coffee", "droplet", "eye", "person-standing"].includes(currentItem.icon || "");
  const effectiveWidth = (isStyledBadge && !expanded) ? POMODORO_BADGE_WIDTH : BADGE_WIDTH;
  const expandedHeight = items.length * PEEK_BADGE_ROW_HEIGHT + PEEK_BADGE_EXPANDED_VERTICAL_PADDING;

  return (
    <AnimatePresence mode="wait">
      {expanded ? (
        <motion.div
          key="expanded"
          initial={{ opacity: 0, scale: 0.97 }}
          animate={{ opacity: 1, scale: 1 }}
          exit={{ opacity: 0, scale: 0.97 }}
          className="pointer-events-auto absolute z-10 cursor-pointer rounded-2xl border border-white/5 bg-black/20 px-3 py-2 backdrop-blur-3xl shadow-2xl"
          style={{
            top: PEEK_BADGE_PADDING,
            left: "50%",
            marginLeft: -(BADGE_WIDTH / 2),
            width: BADGE_WIDTH,
            minHeight: expandedHeight,
          }}
          onClick={onToggle}
        >
          {items.map((item) => (
            <BadgeRow key={`${item.label}-${item.value}`} item={item} compact />
          ))}
        </motion.div>
      ) : (
        <motion.div
          key={`collapsed-${currentItem.icon || currentItem.label}`}
          initial={{ opacity: 0, y: 0 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: 0 }}
          className={`pointer-events-auto absolute z-10 cursor-pointer overflow-visible ${
            isStyledBadge 
            ? "bg-transparent h-12" 
            : "rounded-xl border border-glass-border/60 bg-glass/80 px-3 py-1 h-12 backdrop-blur-md shadow-panel/40"
          }`}
          style={{
            top: PEEK_BADGE_PADDING,
            left: "50%",
            marginLeft: -(effectiveWidth / 2),
            width: effectiveWidth,
            height: PEEK_BADGE_HEIGHT,
          }}
          onClick={isStyledBadge ? undefined : onToggle}
        >
          {isStyledBadge ? (
             <StyledBadge item={currentItem} />
          ) : (
             <BadgeRow item={currentItem} />
          )}
        </motion.div>
      )}
    </AnimatePresence>
  );
}
