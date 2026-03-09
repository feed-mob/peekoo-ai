import { motion, AnimatePresence } from "framer-motion";
import { MessageSquare, CheckSquare, Timer, HeartPulse, Puzzle, Blocks } from "lucide-react";
import type { PanelLabel } from "@/types/window";
import type { PanelWindowStates } from "@/hooks/use-panel-windows";
import type { LucideIcon } from "lucide-react";
import type { PluginPanel } from "@/types/plugin";
import { cn } from "@/lib/utils";
import { getSpriteActionMenuItems } from "./spriteActionMenuLayout";

interface MenuItemConfig {
  label: PanelLabel;
  icon: LucideIcon;
  name: string;
  x: number;
  y: number;
}

function iconForPluginPanel(panel: PluginPanel): LucideIcon {
  if (panel.pluginKey === "health-reminders") {
    return HeartPulse;
  }
  return Puzzle;
}

interface SpriteActionMenuProps {
  panels: PanelWindowStates;
  onTogglePanel: (label: PanelLabel) => void;
  isOpen: boolean;
  pluginPanels?: PluginPanel[];
}

export function SpriteActionMenu({
  panels,
  onTogglePanel,
  isOpen,
  pluginPanels = [],
}: SpriteActionMenuProps) {
  const items: MenuItemConfig[] = getSpriteActionMenuItems(pluginPanels).map((item) => {
    const panel = pluginPanels.find((pluginPanel) => pluginPanel.label === item.label);

    return {
      ...item,
      icon:
        item.label === "panel-chat"
          ? MessageSquare
          : item.label === "panel-tasks"
            ? CheckSquare
            : item.label === "panel-pomodoro"
              ? Timer
              : item.label === "panel-plugins"
                ? Blocks
                : iconForPluginPanel(panel!),
    };
  });

  return (
    <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
      <AnimatePresence>
        {isOpen &&
          items.map((item, index) => {
            const Icon = item.icon;
            const isPanelOpen = panels[item.label]?.isOpen;

            return (
              <motion.button
                key={item.label}
                initial={{ opacity: 0, scale: 0, x: 0, y: 0 }}
                animate={{ opacity: 1, scale: 1, x: item.x, y: item.y }}
                exit={{ opacity: 0, scale: 0, x: 0, y: 0 }}
                transition={{
                  delay: index * 0.05,
                  type: "spring",
                  stiffness: 400,
                  damping: 25,
                }}
                onClick={(e) => {
                  e.stopPropagation();
                  onTogglePanel(item.label);
                }}
                aria-label={item.name}
                className={cn(
                  "group absolute pointer-events-auto flex h-[38px] w-[38px] items-center justify-center rounded-full border transition-colors cursor-pointer",
                  isPanelOpen
                    ? "bg-glow-blue/20 border-glow-blue/40 text-glow-blue"
                    : "bg-glass border-glass-border text-text-secondary hover:text-text-primary hover:bg-space-overlay",
                )}
              >
                <span
                  className={cn(
                    "pointer-events-none absolute bottom-full mb-2 whitespace-nowrap rounded-full border px-2 py-1 text-xs font-medium opacity-0 shadow-panel transition-all duration-150 group-hover:-translate-y-0.5 group-hover:opacity-100 group-focus-visible:-translate-y-0.5 group-focus-visible:opacity-100",
                    isPanelOpen
                      ? "bg-space-overlay border-glow-blue/40 text-text-primary"
                      : "bg-glass border-glass-border text-text-secondary",
                  )}
                >
                  {item.name}
                </span>
                <Icon size={16} />
              </motion.button>
            );
          })}
      </AnimatePresence>
    </div>
  );
}
