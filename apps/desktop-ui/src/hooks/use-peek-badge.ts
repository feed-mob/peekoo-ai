import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import {
  PEEK_BADGES_EVENT,
  PeekBadgesPayloadSchema,
  type PeekBadgeItem,
} from "@/types/peek-badge";

const ROTATION_INTERVAL_MS = 5000;
const COUNTDOWN_TICK_MS = 1000;

function formatCountdown(seconds: number, isPrecise: boolean = false): string {
  if (seconds <= 0) return isPrecise ? "00:00" : "now";
  
  if (isPrecise) {
     const mins = Math.floor(seconds / 60);
     const secs = seconds % 60;
     return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
  }

  const minutes = Math.max(1, Math.floor(seconds / 60));
  if (minutes < 60) return `~${minutes} min`;
  const hours = Math.floor(minutes / 60);
  const remainder = minutes % 60;
  return remainder === 0 ? `~${hours} hr` : `~${hours} hr ${remainder} min`;
}

export function usePeekBadge() {
  const [items, setItems] = useState<PeekBadgeItem[]>([]);
  const [currentIndex, setCurrentIndex] = useState(0);
  const [expanded, setExpanded] = useState(false);
  const snapshotRef = useRef<{ items: PeekBadgeItem[]; at: number }>({
    items: [],
    at: Date.now(),
  });

  // Listen for backend push events, then signal the backend that the UI is
  // ready to receive badge updates.
  useEffect(() => {
    const unlisten = listen(PEEK_BADGES_EVENT, (event) => {
      const parsed = PeekBadgesPayloadSchema.safeParse(event.payload);
      if (!parsed.success) return;

      snapshotRef.current = { items: parsed.data, at: Date.now() };
      setItems(parsed.data);
      setCurrentIndex((prev) =>
        parsed.data.length > 0 ? prev % parsed.data.length : 0,
      );
    });

    // Signal the backend that badge listeners are registered.
    void invoke("ui_ready");

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Local countdown tick: decrement countdown_secs every second
  useEffect(() => {
    if (items.length === 0) return;

    const id = setInterval(() => {
      const nowEpoch = Math.floor(Date.now() / 1000);
      
      const ticked = snapshotRef.current.items.map((item) => {
        const remaining = item.target_epoch_secs != null 
          ? Math.max(0, item.target_epoch_secs - nowEpoch)
          : null;

        return {
          ...item,
          value: remaining !== null
            ? formatCountdown(
                remaining, 
                ["brain", "coffee", "droplet", "eye", "person-standing"].includes(item.icon || "")
              )
            : item.value,
        };
      });
      setItems(ticked);
    }, COUNTDOWN_TICK_MS);

    return () => clearInterval(id);
  }, [items.length]);

  // Auto-rotate through items
  useEffect(() => {
    if (items.length <= 1 || expanded) return;

    const id = setInterval(() => {
      setCurrentIndex((prev) => (prev + 1) % items.length);
    }, ROTATION_INTERVAL_MS);

    return () => clearInterval(id);
  }, [items.length, expanded]);

  const toggleExpanded = useCallback(() => {
    setExpanded((prev) => !prev);
  }, []);

  const collapse = useCallback(() => {
    setExpanded(false);
  }, []);

  const currentItem = items.length > 0 ? items[currentIndex] ?? items[0] : null;

  return {
    items,
    currentItem,
    currentIndex,
    expanded,
    toggleExpanded,
    collapse,
  };
}
