import { getCurrentWindow } from "@tauri-apps/api/window";
import { X } from "lucide-react";
import type { ReactNode } from "react";
import { emitPetReaction } from "@/lib/pet-events";
import { motion } from "framer-motion";

interface PanelShellProps {
  title: string;
  children: ReactNode;
  showCloseButton?: boolean;
}

export function PanelShell({ title, children, showCloseButton = true }: PanelShellProps) {
  const handleClose = async () => {
    console.log("PanelShell close clicked");
    await emitPetReaction("panel-closed");
    const win = getCurrentWindow();
    console.log("PanelShell window:", win.label);
    await win.close();
  };

  const dragRegionClass = showCloseButton
    ? "absolute inset-0 right-12 cursor-grab active:cursor-grabbing"
    : "absolute inset-0 cursor-grab active:cursor-grabbing";

  return (
    <div className="w-full h-screen flex flex-col bg-glass backdrop-blur-2xl border border-glass-border rounded-panel overflow-hidden shadow-panel">
      {/* Title bar */}
      <div className="relative flex items-center justify-between h-12 px-4 select-none shrink-0 z-20">
        {/* Drag region - keep it off the close button */}
        <div data-tauri-drag-region className={dragRegionClass} />
        {title ? (
          <span className="relative z-10 text-sm font-semibold text-text-primary opacity-80 tracking-wide pointer-events-none">
            {title}
          </span>
        ) : (
          <span className="relative z-10 text-sm font-semibold text-text-primary opacity-80 tracking-wide pointer-events-none" />
        )}
        {showCloseButton ? (
          <motion.button
            onClick={handleClose}
            whileHover={{ scale: 1.1 }}
            whileTap={{ scale: 0.9 }}
            className="relative z-10 p-1.5 rounded-full hover:bg-space-surface text-text-muted hover:text-text-primary transition-colors cursor-pointer pointer-events-auto"
            data-tauri-drag-region={false}
          >
            <X size={16} />
          </motion.button>
        ) : null}
      </div>

      {/* Panel content */}
      <div className="relative z-10 flex-1 min-w-0 px-panel-padding pb-panel-padding overflow-y-auto overflow-x-hidden">
        {children}
      </div>
    </div>
  );
}
