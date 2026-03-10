import { getCurrentWindow } from "@tauri-apps/api/window";
import { X } from "lucide-react";
import type { ReactNode } from "react";
import { emitPetReaction } from "@/lib/pet-events";
import { motion } from "framer-motion";

interface PanelShellProps {
  title: string;
  children: ReactNode;
}

export function PanelShell({ title, children }: PanelShellProps) {
  const handleClose = async () => {
    await emitPetReaction("panel-closed");
    const win = getCurrentWindow();
    await win.close();
  };

  return (
    <div className="w-full h-screen flex flex-col bg-glass backdrop-blur-2xl border border-glass-border rounded-panel overflow-hidden shadow-panel">
      {/* Title bar / drag region */}
      <div
        data-tauri-drag-region
        className="flex items-center justify-between h-12 px-4 select-none shrink-0"
      >
        <span
          data-tauri-drag-region
          className="text-sm font-semibold text-text-primary opacity-80 tracking-wide"
        >
          {title}
        </span>
        <motion.button
          onClick={handleClose}
          whileHover={{ scale: 1.1 }}
          whileTap={{ scale: 0.9 }}
          className="p-1.5 rounded-full hover:bg-space-surface text-text-muted hover:text-text-primary transition-colors cursor-pointer"
        >
          <X size={16} />
        </motion.button>
      </div>

      {/* Panel content */}
      <div className="flex-1 min-w-0 px-panel-padding pb-panel-padding overflow-y-auto overflow-x-hidden">{children}</div>
    </div>
  );
}
