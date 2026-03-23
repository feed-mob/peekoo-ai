import { X, CheckCircle, AlertCircle, Info } from "lucide-react";
import type { Toast } from "../hooks/use-toast";

interface ErrorToastProps {
  toasts: Toast[];
  onRemove: (id: string) => void;
}

const icons = {
  success: CheckCircle,
  error: AlertCircle,
  info: Info,
};

const styles = {
  success: "bg-green-500/20 border-green-500/40 text-green-400",
  error: "bg-red-500/20 border-red-500/40 text-red-400",
  info: "bg-blue-500/20 border-blue-500/40 text-blue-400",
};

export function ErrorToast({ toasts, onRemove }: ErrorToastProps) {
  if (toasts.length === 0) return null;

  return (
    <div className="fixed bottom-4 right-4 z-50 flex flex-col gap-2">
      {toasts.map((toast) => {
        const Icon = icons[toast.type];
        return (
          <div
            key={toast.id}
            className={`flex items-center gap-2 px-4 py-3 rounded-lg border shadow-lg min-w-[300px] animate-in slide-in-from-right ${styles[toast.type]}`}
            role="alert"
          >
            <Icon size={18} />
            <span className="flex-1 text-sm font-medium">{toast.message}</span>
            <button
              onClick={() => onRemove(toast.id)}
              className="p-1 rounded hover:bg-black/20 transition-colors"
              aria-label="Dismiss"
            >
              <X size={14} />
            </button>
          </div>
        );
      })}
    </div>
  );
}
