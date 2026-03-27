import { useState, useMemo } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  MessageSquare,
  CheckSquare,
  Timer,
  HeartPulse,
  Puzzle,
  Blocks,
} from "lucide-react";
import type { PanelLabel } from "@/types/window";
import type { PanelWindowStates } from "@/hooks/use-panel-windows";
import type { LucideIcon } from "lucide-react";
import type { PluginPanel, PluginSummary } from "@/types/plugin";
import { cn } from "@/lib/utils";
import { getSpriteActionMenuItems } from "./spriteActionMenuLayout";
import { computePluginsPopupPosition } from "./spriteActionMenuPopupPosition";

interface MenuItemConfig {
  label: PanelLabel;
  icon: LucideIcon;
  name: string;
  x: number;
  y: number;
}

const PLUGINS_POPUP_TAIL_SIZE = 12;
const PLUGINS_POPUP_TAIL_PADDING = 16;

/**
 * Vertical offset (px) from the container center to the popup's bottom edge.
 * The action-menu buttons sit at ROW_Y = 72 and are 38px tall, so their top
 * is at roughly y = 53.  We add a small gap.
 */
const POPUP_BOTTOM_OFFSET = 46;

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
  installedPlugins?: PluginSummary[];
}

export function SpriteActionMenu({
  panels,
  onTogglePanel,
  isOpen,
  pluginPanels = [],
  installedPlugins = [],
}: SpriteActionMenuProps) {
  const [pluginsPopupOpen, setPluginsPopupOpen] = useState(false);
  const enabledPluginPanels = pluginPanels.filter((panel) =>
    installedPlugins.some(
      (plugin) => plugin.enabled && plugin.pluginKey === panel.pluginKey,
    ),
  );

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

  // Compute the tail position.  The plugins button's x offset from center
  // is known from the layout.  Since the popup is always CSS-centered, we
  // only need the tail's horizontal offset within the popup.
  const tailOffsetX = useMemo(() => {
    if (!pluginsItem) return null;
    // popup min-width is 180px; actual width may be wider but 180 is our
    // design target and the tail calculation is stable regardless.
    const { tailOffsetX: tx } = computePluginsPopupPosition({
      popupWidth: 180,
      buttonOffsetX: pluginsItem.x,
      tailPadding: PLUGINS_POPUP_TAIL_PADDING,
    });
    return tx;
  }, [pluginsItem]);

  const showPopup = isOpen && pluginsPopupOpen;

  return (
    <div
      className="absolute inset-0 flex items-center justify-center pointer-events-none"
      onClick={() => setPluginsPopupOpen(false)}
    >
      {/* ── Plugins popup ─────────────────────────────────────────────── */}
      {/* Rendered outside the motion.div items so Framer Motion cannot   */}
      {/* override its CSS transform.  Centering uses flexbox (not       */}
      {/* transform) so there is nothing for Framer Motion to stomp.     */}
      <AnimatePresence>
        {showPopup && (
            <motion.div
              key="plugins-popup"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              transition={{ duration: 0.15 }}
              className="absolute inset-x-0 flex justify-center z-50 pointer-events-none"
              style={{ bottom: `calc(50% + ${POPUP_BOTTOM_OFFSET}px)` }}
            >
              <div
                className="relative min-w-[180px] pointer-events-auto"
                onClick={(e) => e.stopPropagation()}
              >
                {/* Tail / arrow */}
                <div
                  aria-hidden="true"
                  className="absolute z-0 rotate-45 bg-glass backdrop-blur-xl border-r border-b border-glass-border"
                  style={{
                    width: `${PLUGINS_POPUP_TAIL_SIZE}px`,
                    height: `${PLUGINS_POPUP_TAIL_SIZE}px`,
                    left:
                      tailOffsetX === null
                        ? `calc(50% - ${PLUGINS_POPUP_TAIL_SIZE / 2}px)`
                        : `${tailOffsetX - PLUGINS_POPUP_TAIL_SIZE / 2}px`,
                    bottom: `${-(PLUGINS_POPUP_TAIL_SIZE / 2)}px`,
                  }}
                />

                {/* Popup body */}
                <div className="relative z-10 rounded-lg border border-glass-border bg-glass backdrop-blur-xl shadow-panel p-2">
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
                    <Blocks size={16} className="shrink-0" />
                    <span className="text-xs font-medium whitespace-nowrap">Plugins</span>
                  </button>

                  {enabledPluginPanels.map((panel) => {
                    const panelLabel = panel.label;
                    const PanelIcon = iconForPluginPanel(panel);
                    const isPanelOpen = panels[panelLabel]?.isOpen;

                    return (
                      <button
                        key={panel.label}
                        onClick={(e) => {
                          e.stopPropagation();
                          setPluginsPopupOpen(false);
                          onTogglePanel(panelLabel);
                        }}
                        className={cn(
                          "flex w-full items-center gap-2 px-3 py-2 rounded-md cursor-pointer transition-colors",
                          isPanelOpen
                            ? "bg-space-overlay text-text-primary"
                            : "text-text-secondary hover:bg-space-overlay hover:text-text-primary",
                        )}
                      >
                        <PanelIcon size={16} className="shrink-0" />
                        <span className="text-xs font-medium whitespace-nowrap">
                          {panel.title}
                        </span>
                      </button>
                    );
                  })}
                </div>
              </div>
            </motion.div>
        )}
      </AnimatePresence>

      {/* ── Action menu buttons ───────────────────────────────────────── */}
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
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    if (isPluginsButton) {
                      setPluginsPopupOpen((prev) => !prev);
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
                      isPluginsButton && pluginsPopupOpen && "hidden",
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
