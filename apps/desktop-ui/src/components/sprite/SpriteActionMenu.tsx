import { motion, AnimatePresence } from "framer-motion";
import { MessageSquare, CheckSquare, Timer } from "lucide-react";
import type { PanelLabel } from "@/types/window";
import type { PanelWindowStates } from "@/hooks/use-panel-windows";
import type { LucideIcon } from "lucide-react";

interface MenuItemConfig {
  label: PanelLabel;
  icon: LucideIcon;
  name: string;
  angle: number;
}

const MENU_ITEMS: ReadonlyArray<MenuItemConfig> = [
  { label: "panel-chat", icon: MessageSquare, name: "Chat", angle: -60 },
  { label: "panel-tasks", icon: CheckSquare, name: "Tasks", angle: 0 },
  { label: "panel-pomodoro", icon: Timer, name: "Pomodoro", angle: 60 },
] as const;

const RADIUS = 70;

interface SpriteActionMenuProps {
  panels: PanelWindowStates;
  onTogglePanel: (label: PanelLabel) => void;
  isOpen: boolean;
}

export function SpriteActionMenu({
  panels,
  onTogglePanel,
  isOpen,
}: SpriteActionMenuProps) {
  return (
    <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
      <AnimatePresence>
        {isOpen &&
          MENU_ITEMS.map((item, index) => {
            const rad = (item.angle * Math.PI) / 180;
            const x = Math.cos(rad) * RADIUS;
            const y = Math.sin(rad) * RADIUS;
            const Icon = item.icon;
            const isPanelOpen = panels[item.label]?.isOpen;

            return (
              <motion.button
                key={item.label}
                initial={{ opacity: 0, scale: 0, x: 0, y: 0 }}
                animate={{ opacity: 1, scale: 1, x, y: y - 30 }}
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
                className={`absolute pointer-events-auto flex items-center gap-1.5 px-3 py-1.5 rounded-full border transition-colors cursor-pointer ${
                  isPanelOpen
                    ? "bg-glow-blue/20 border-glow-blue/40 text-glow-blue"
                    : "bg-glass border-glass-border text-text-secondary hover:text-text-primary hover:bg-space-overlay"
                }`}
              >
                <Icon size={14} />
                <span className="text-xs font-medium">{item.name}</span>
              </motion.button>
            );
          })}
      </AnimatePresence>
    </div>
  );
}
