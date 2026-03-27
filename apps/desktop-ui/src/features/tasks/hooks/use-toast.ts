import { useState, useCallback } from "react";

export type ToastType = "success" | "error" | "info";

export interface Toast {
  id: string;
  type: ToastType;
  message: string;
  duration?: number;
}

export function useToast() {
  const [toasts, setToasts] = useState<Toast[]>([]);

  const addToast = useCallback(
    ({
      type,
      message,
      duration = 3000,
    }: {
      type: ToastType;
      message: string;
      duration?: number;
    }) => {
      const id = Math.random().toString(36).substring(2, 9);
      const toast: Toast = { id, type, message, duration };

      setToasts((prev) => [...prev, toast]);

      // Auto-remove after duration
      setTimeout(() => {
        setToasts((prev) => prev.filter((t) => t.id !== id));
      }, duration);

      return id;
    },
    []
  );

  const removeToast = useCallback((id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  const success = useCallback(
    (message: string, duration?: number) =>
      addToast({ type: "success", message, duration }),
    [addToast]
  );

  const error = useCallback(
    (message: string, duration?: number) =>
      addToast({ type: "error", message, duration }),
    [addToast]
  );

  const info = useCallback(
    (message: string, duration?: number) =>
      addToast({ type: "info", message, duration }),
    [addToast]
  );

  return {
    toasts,
    addToast,
    removeToast,
    success,
    error,
    info,
  };
}
