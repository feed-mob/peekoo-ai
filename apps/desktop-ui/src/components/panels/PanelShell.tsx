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
    <motion.div
      initial={{ opacity: 0, scale: 0.95, y: 20 }}
      animate={{ opacity: 1, scale: 1, y: 0 }}
      exit={{ opacity: 0, scale: 0.95, y: 20 }}
      transition={{ type: "spring", stiffness: 300, damping: 30 }}
      className="w-full h-screen flex flex-col bg-glass backdrop-blur-xl border border-glass-border rounded-panel overflow-hidden shadow-2xl"
    >
      {/* Title bar / drag region */}
      <div
        data-tauri-drag-region
        className="flex items-center justify-between h-11 px-4 select-none shrink-0 border-b border-glass-border/50"
      >
        <span
          data-tauri-drag-region
          className="text-sm font-semibold text-text-primary tracking-wide"
        >
          {title}
        </span>
        <motion.button
          onClick={handleClose}
          whileHover={{ scale: 1.1, backgroundColor: "oklch(0.60 0.18 25 / 0.15)" }}
          whileTap={{ scale: 0.9 }}
          className="p-1.5 rounded-lg hover:bg-space-surface text-text-muted hover:text-color-danger transition-colors cursor-pointer"
          aria-label="Close panel"
        >
          <X size={16} />
        </motion.button>
      </div>

      {/* Panel content */}
      <div className="flex-1 min-w-0 p-5 overflow-y-auto overflow-x-hidden">{children}</div>
    </motion.div>
  );
}
