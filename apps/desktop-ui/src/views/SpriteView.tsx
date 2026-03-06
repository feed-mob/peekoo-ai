import { useState, useCallback, useEffect, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Sprite } from "@/components/sprite/Sprite";
import { SpriteActionMenu } from "@/components/sprite/SpriteActionMenu";
import { useSpriteState } from "@/hooks/use-sprite-state";
import { usePanelWindows } from "@/hooks/use-panel-windows";
import { useSpriteReactions } from "@/hooks/use-sprite-reactions";
import type { AnimationType, SpriteState } from "@/types/sprite";

// Duration (ms) a reaction-triggered mood override stays active before reverting
const MOOD_OVERRIDE_DURATION_MS = 3000;
// If the OS drag completes faster than this (ms), the user just clicked
const CLICK_TIME_THRESHOLD_MS = 150;

export default function SpriteView() {
  const spriteState = useSpriteState();
  const { panels, togglePanel, expandForMenu, shrinkToSprite } =
    usePanelWindows();
  const [menuOpen, setMenuOpen] = useState(false);
  const [randomTrigger, setRandomTrigger] = useState(0);
  const [moodOverride, setMoodOverride] = useState<string | null>(null);
  const [dragAnimation, setDragAnimation] = useState<AnimationType | null>(null);
  const moodResetTimerRef = useRef<number | null>(null);

  const clearMoodResetTimer = useCallback(() => {
    if (moodResetTimerRef.current !== null) {
      window.clearTimeout(moodResetTimerRef.current);
      moodResetTimerRef.current = null;
    }
  }, []);

  const handleMoodChange = useCallback((mood: string, sticky: boolean) => {
    clearMoodResetTimer();
    setMoodOverride(mood);

    if (!sticky) {
      moodResetTimerRef.current = window.setTimeout(() => {
        setMoodOverride(null);
        moodResetTimerRef.current = null;
      }, MOOD_OVERRIDE_DURATION_MS);
    }
  }, [clearMoodResetTimer]);

  useEffect(() => {
    return () => {
      clearMoodResetTimer();
    };
  }, [clearMoodResetTimer]);

  useSpriteReactions({ onMoodChange: handleMoodChange });

  const effectiveSpriteState: SpriteState = moodOverride
    ? { ...spriteState, mood: moodOverride }
    : spriteState;

  // Start OS drag on mousedown. We set the dragging animation first, then
  // yield a frame so React can paint before startDragging() hands control
  // to the OS (which freezes the JS event loop until the drag ends).
  // If the drag completes very quickly, the user just clicked.
  const handleMouseDown = useCallback(async (e: React.MouseEvent) => {
    if (e.button !== 0) return; // only primary button
    e.stopPropagation();
    setDragAnimation("dragging");
    try {
      // Yield to let React flush the dragging animation before OS takes over
      await new Promise((r) => requestAnimationFrame(r));
      const start = performance.now();
      await getCurrentWindow().startDragging();
      const elapsed = performance.now() - start;
      if (elapsed < CLICK_TIME_THRESHOLD_MS) {
        // Drag ended almost instantly — user just clicked
        setRandomTrigger((prev) => prev + 1);
      }
    } catch (error) {
      console.error("Failed to start dragging sprite window", error);
    } finally {
      setDragAnimation(null);
    }
  }, []);

  // Right click: toggle menu
  const handleContextMenu = useCallback(
    async (e: React.MouseEvent) => {
      e.preventDefault();
      if (menuOpen) {
        setMenuOpen(false);
        await shrinkToSprite();
      } else {
        await expandForMenu();
        setMenuOpen(true);
      }
    },
    [menuOpen, expandForMenu, shrinkToSprite],
  );

  const handleTogglePanel = useCallback(
    async (label: Parameters<typeof togglePanel>[0]) => {
      await togglePanel(label);
      setMenuOpen(false);
      await shrinkToSprite();
    },
    [togglePanel, shrinkToSprite],
  );

  return (
    <div
      className="w-full h-full bg-transparent"
      data-tauri-drag-region
    >
      <div className="relative flex items-center justify-center w-full h-full">
        <div
          onMouseDown={handleMouseDown}
          onContextMenu={handleContextMenu}
          className="cursor-pointer"
        >
          <Sprite
            state={effectiveSpriteState}
            randomTrigger={randomTrigger}
            animationOverride={dragAnimation}
          />
        </div>

        <SpriteActionMenu
          panels={panels}
          onTogglePanel={handleTogglePanel}
          isOpen={menuOpen}
        />
      </div>
    </div>
  );
}
