import { getCurrentWindow } from "@tauri-apps/api/window";
import { X } from "lucide-react";
import type { ReactNode } from "react";
import { emitPetReaction } from "@/lib/pet-events";

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
    <div className="w-full h-screen flex flex-col bg-glass backdrop-blur-xl border border-glass-border rounded-panel overflow-hidden">
      {/* Title bar / drag region */}
      <div
        data-tauri-drag-region
        className="flex items-center justify-between h-10 px-3 border-b border-glass-border select-none shrink-0"
      >
        <span
          data-tauri-drag-region
          className="text-sm font-medium text-text-secondary"
        >
          {title}
        </span>
        <button
          onClick={handleClose}
          className="p-1 rounded hover:bg-danger/20 text-text-muted hover:text-danger transition-colors cursor-pointer"
        >
          <X size={14} />
        </button>
      </div>

      {/* Panel content */}
      <div className="flex-1 min-w-0 p-panel-padding overflow-y-auto overflow-x-hidden">{children}</div>
    </div>
  );
}
