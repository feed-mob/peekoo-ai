import { useState } from "react";
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
  const [pluginsPopupOpen, setPluginsPopupOpen] = useState(false);

  const items: MenuItemConfig[] = getSpriteActionMenuItems().map((item) => {
    return {
      ...item,
      icon:
        item.label === "panel-chat"
          ? MessageSquare
          : item.label === "panel-tasks"
            ? CheckSquare
            : item.label === "panel-pomodoro"
              ? Timer
              : Blocks,
    };
  });

  const anyPluginPanelOpen =
    panels["panel-plugins"]?.isOpen ||
    pluginPanels.some((p) => panels[p.label]?.isOpen);

  const pluginsItem = items.find((item) => item.label === "panel-plugins");

  return (
    <div
      className="absolute inset-0 flex items-center justify-center pointer-events-none"
      onClick={() => setPluginsPopupOpen(false)}
    >
      <AnimatePresence>
        {isOpen &&
          items.map((item, index) => {
            const Icon = item.icon;
            const isPanelOpen = panels[item.label]?.isOpen;
            const isPluginsButton = item.label === "panel-plugins";
            const isActive = isPluginsButton
              ? pluginsPopupOpen || anyPluginPanelOpen
              : isPanelOpen;

            return (
              <motion.div
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
                className="absolute"
              >
                {isPluginsButton && pluginsPopupOpen && pluginsItem && (
                  <motion.div
                    initial={{ opacity: 0, y: 8 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, y: 8 }}
                    transition={{ duration: 0.15 }}
                    onClick={(e) => e.stopPropagation()}
                    className="absolute bottom-full mb-3 left-1/2 -translate-x-1/2 bg-glass border border-glass-border rounded-lg shadow-panel p-2 min-w-[160px] pointer-events-auto"
                  >
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        setPluginsPopupOpen(false);
                        onTogglePanel("panel-plugins");
                      }}
                      className={cn(
                        "flex w-full items-center gap-2 px-3 py-2 rounded-md cursor-pointer transition-colors",
                        panels["panel-plugins"]?.isOpen
                          ? "bg-space-overlay text-text-primary"
                          : "text-text-secondary hover:bg-space-overlay hover:text-text-primary",
                      )}
                    >
                      <Blocks size={16} />
                      <span className="text-xs font-medium">Plugin Manager</span>
                    </button>

                    {pluginPanels.map((panel) => {
                      const PanelIcon = iconForPluginPanel(panel);
                      const isOpen = panels[panel.label]?.isOpen;

                      return (
                        <button
                          key={panel.label}
                          onClick={(e) => {
                            e.stopPropagation();
                            setPluginsPopupOpen(false);
                            onTogglePanel(panel.label);
                          }}
                          className={cn(
                            "flex w-full items-center gap-2 px-3 py-2 rounded-md cursor-pointer transition-colors",
                            isOpen
                              ? "bg-space-overlay text-text-primary"
                              : "text-text-secondary hover:bg-space-overlay hover:text-text-primary",
                          )}
                        >
                          <PanelIcon size={16} />
                          <span className="text-xs font-medium">{panel.title}</span>
                        </button>
                      );
                    })}
                  </motion.div>
                )}

                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    if (isPluginsButton) {
                      if (pluginPanels.length === 0) {
                        onTogglePanel("panel-plugins");
                      } else {
                        setPluginsPopupOpen((prev) => !prev);
                      }
                    } else {
                      onTogglePanel(item.label);
                    }
                  }}
                  aria-label={item.name}
                  className={cn(
                    "group pointer-events-auto flex h-[38px] w-[38px] items-center justify-center rounded-full border transition-colors cursor-pointer",
                    isActive
                      ? "bg-glow-blue/20 border-glow-blue/40 text-glow-blue"
                      : "bg-glass border-glass-border text-text-secondary hover:text-text-primary hover:bg-space-overlay",
                  )}
                >
                  <span
                    className={cn(
                      "pointer-events-none absolute bottom-full mb-2 whitespace-nowrap rounded-full border px-2 py-1 text-xs font-medium opacity-0 shadow-panel transition-all duration-150 group-hover:-translate-y-0.5 group-hover:opacity-100 group-focus-visible:-translate-y-0.5 group-focus-visible:opacity-100",
                      isActive
                        ? "bg-space-overlay border-glow-blue/40 text-text-primary"
                        : "bg-glass border-glass-border text-text-secondary",
                    )}
                  >
                    {item.name}
                  </span>
                  <Icon size={16} />
                </button>
              </motion.div>
            );
          })}
      </AnimatePresence>
    </div>
  );
}
