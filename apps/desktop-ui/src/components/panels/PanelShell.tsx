import { getCurrentWindow } from "@tauri-apps/api/window";
import { X } from "lucide-react";
import type { MouseEvent, ReactNode } from "react";
import { emitPetReaction } from "@/lib/pet-events";
import { motion } from "framer-motion";

interface PanelShellProps {
  title: string;
  children: ReactNode;
}

type ResizeDirection =
  | "North"
  | "South"
  | "East"
  | "West"
  | "NorthEast"
  | "NorthWest"
  | "SouthEast"
  | "SouthWest";

const RESIZE_HANDLE_CLASSES: Record<ResizeDirection, string> = {
  North: "top-0 left-2 right-2 h-1 cursor-n-resize",
  South: "bottom-0 left-2 right-2 h-1 cursor-s-resize",
  East: "right-0 top-2 bottom-2 w-1 cursor-e-resize",
  West: "left-0 top-2 bottom-2 w-1 cursor-w-resize",
  NorthEast: "top-0 right-0 h-3 w-3 cursor-ne-resize",
  NorthWest: "top-0 left-0 h-3 w-3 cursor-nw-resize",
  SouthEast: "bottom-0 right-0 h-3 w-3 cursor-se-resize",
  SouthWest: "bottom-0 left-0 h-3 w-3 cursor-sw-resize",
};

export function PanelShell({ title, children }: PanelShellProps) {
  const handleClose = async () => {
    await emitPetReaction("panel-closed");
    const win = getCurrentWindow();
    await win.close();
  };

  const handleResizeStart =
    (direction: ResizeDirection) =>
    async (event: MouseEvent<HTMLDivElement>) => {
      event.preventDefault();
      event.stopPropagation();
      const win = getCurrentWindow();
      await win.startResizeDragging(direction);
    };

  return (
    <div className="relative w-full h-screen flex flex-col bg-glass backdrop-blur-2xl border border-glass-border rounded-panel overflow-hidden shadow-panel">
      {(
        Object.entries(RESIZE_HANDLE_CLASSES) as [ResizeDirection, string][]
      ).map(([direction, className]) => (
        <div
          key={direction}
          className={`absolute z-30 ${className}`}
          onMouseDown={handleResizeStart(direction)}
        />
      ))}

      {/* Title bar */}
      <div className="relative flex items-center justify-between h-12 px-4 select-none shrink-0 z-20">
        {/* Drag region - keep it off the close button */}
        <div
          data-tauri-drag-region
          className="absolute inset-0 right-12 cursor-grab active:cursor-grabbing"
        />
        {title ? (
          <span className="relative z-10 text-sm font-semibold text-text-primary opacity-80 tracking-wide pointer-events-none">
            {title}
          </span>
        ) : (
          <span className="relative z-10 text-sm font-semibold text-text-primary opacity-80 tracking-wide pointer-events-none" />
        )}
        <motion.button
          onClick={handleClose}
          whileHover={{ scale: 1.1 }}
          whileTap={{ scale: 0.9 }}
          className="relative z-10 p-1.5 rounded-full hover:bg-space-surface text-text-muted hover:text-text-primary transition-colors cursor-pointer pointer-events-auto"
          data-tauri-drag-region={false}
        >
          <X size={16} />
        </motion.button>
      </div>

      {/* Panel content */}
      <div className="relative z-10 flex-1 min-w-0 px-panel-padding pb-panel-padding overflow-y-auto overflow-x-hidden">
        {children}
      </div>
    </div>
  );
}
