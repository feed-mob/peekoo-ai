import { useState, useCallback, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Sprite } from "@/components/sprite/Sprite";
import { SpriteActionMenu } from "@/components/sprite/SpriteActionMenu";
import { SpriteBubble } from "@/components/sprite/SpriteBubble";
import { SpritePeekBadge } from "@/components/sprite/SpritePeekBadge";
import { usePeekBadge } from "@/hooks/use-peek-badge";
import { useSpriteBubble } from "@/hooks/use-sprite-bubble";
import { getSpriteWindowSize } from "@/lib/sprite-bubble-layout";
import { useSpriteState } from "@/hooks/use-sprite-state";
import { usePanelWindows } from "@/hooks/use-panel-windows";
import { useSpriteReactions } from "@/hooks/use-sprite-reactions";
import {
  SPRITE_BUBBLE_DURATION_MS,
  SPRITE_BUBBLE_EVENT,
  SpriteBubblePayloadSchema,
} from "@/types/sprite-bubble";
import type { AnimationType, SpriteState } from "@/types/sprite";

// Duration (ms) a reaction-triggered mood override stays active before reverting
const MOOD_OVERRIDE_DURATION_MS = 3000;

export default function SpriteView() {
  const spriteState = useSpriteState();
  const { payload: bubblePayload, visible: bubbleVisible, showBubble, clearBubble } = useSpriteBubble();
  const { items: badgeItems, currentItem: badgeCurrentItem, expanded: badgeExpanded, toggleExpanded: toggleBadgeExpanded, collapse: collapseBadge } = usePeekBadge();
  const { panels, pluginPanels, installedPlugins, togglePanel } = usePanelWindows();
  const [menuOpen, setMenuOpen] = useState(false);
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
      clearBubble();
    };
  }, [clearBubble, clearMoodResetTimer]);

  // Track the previous extraTop so we can compute the delta for position adjustment.
  const prevExtraTopRef = useRef(0);

  // Auto-expand/shrink the main window when bubble visibility or menu state changes.
  // We invoke a Rust command instead of JS setSize() because resizable:false blocks the JS API.
  useEffect(() => {
    const nextSize = getSpriteWindowSize({
      menuOpen,
      bubbleOpen: bubblePayload !== null && bubbleVisible,
      peekBadgeItemCount: badgeItems.length,
      peekBadgeExpanded: badgeExpanded,
    });
    const deltaTop = nextSize.extraTop - prevExtraTopRef.current;
    prevExtraTopRef.current = nextSize.extraTop;
    void invoke("resize_sprite_window", {
      width: nextSize.width,
      height: nextSize.height,
      deltaTop,
    });
  }, [bubblePayload, bubbleVisible, menuOpen, badgeItems.length, badgeExpanded]);

  useEffect(() => {
    const unlisten = listen(SPRITE_BUBBLE_EVENT, (event) => {
      const parsed = SpriteBubblePayloadSchema.safeParse(event.payload);
      if (!parsed.success) {
        return;
      }

      collapseBadge();
      showBubble(parsed.data);

      clearMoodResetTimer();
      setMoodOverride("reminder");
      moodResetTimerRef.current = window.setTimeout(() => {
        setMoodOverride(null);
        moodResetTimerRef.current = null;
      }, SPRITE_BUBBLE_DURATION_MS);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [clearMoodResetTimer, collapseBadge, showBubble]);

  useSpriteReactions({ onMoodChange: handleMoodChange });

  const effectiveSpriteState: SpriteState = moodOverride
    ? { ...spriteState, mood: moodOverride }
    : spriteState;

  // Start OS drag on mousedown. We set the dragging animation first, then
  // yield a frame so React can paint before startDragging() hands control
  // to the OS (which freezes the JS event loop until the drag ends).
  // If the drag completes very quickly, the user just clicked.
  // Clear drag animation ONLY when mouse is released
  useEffect(() => {
    const handleGlobalMouseUp = () => {
      setDragAnimation(null);
    };
    
    // We only listen to mouseup. 
    // We EXCLUDE 'blur' because startDragging often causes the window to lose focus on Windows,
    // which was likely causing the premature reset to Idle animation.
    window.addEventListener("mouseup", handleGlobalMouseUp);
    
    return () => {
      window.removeEventListener("mouseup", handleGlobalMouseUp);
    };
  }, []);

  // Start OS drag on mousedown.
  const handleMouseDown = useCallback(async (e: React.MouseEvent) => {
    if (e.button !== 0) return; // only primary button
    e.stopPropagation();
    
    // 1. Immediately switch to dragging animation
    setDragAnimation("dragging");
    
    try {
      // 2. Give the browser more time (200ms) to finish React rendering AND paint the canvas
      // frame before the OS takes over the thread. Windows OS drag is very aggressive.
      await new Promise((r) => setTimeout(r, 200));
      await new Promise((r) => requestAnimationFrame(() => r(null)));

      // 3. Start the actual OS drag
      await getCurrentWindow().startDragging();
      
      // Removed CLICK_TIME_THRESHOLD_MS logic as requested in the simpler version
    } catch (error) {
      console.error("Failed to start dragging sprite window", error);
      setDragAnimation(null);
    }
    // We DO NOT setDragAnimation(null) in a finally block here.
    // The global mouseup listener above is responsible for that.
  }, []);

  // Right click: toggle menu
  const handleContextMenu = useCallback(
    async (e: React.MouseEvent) => {
      e.preventDefault();
      if (menuOpen) {
        setMenuOpen(false);
      } else {
        setMenuOpen(true);
      }
    },
    [menuOpen],
  );

  const handleTogglePanel = useCallback(
    async (label: Parameters<typeof togglePanel>[0]) => {
      await togglePanel(label);
      setMenuOpen(false);
    },
    [togglePanel],
  );

  return (
    <div
      className="w-full h-full bg-transparent"
    >
      <div className="relative flex items-center justify-center w-full h-full">
        <div
          onMouseDown={handleMouseDown}
          onContextMenu={handleContextMenu}
          className="cursor-pointer"
        >
          <Sprite
            state={effectiveSpriteState}
            animationOverride={dragAnimation}
          />
        </div>

        <SpriteActionMenu
          panels={panels}
          onTogglePanel={handleTogglePanel}
          isOpen={menuOpen}
          pluginPanels={pluginPanels}
          installedPlugins={installedPlugins}
        />
        <SpritePeekBadge
          items={badgeItems}
          currentItem={badgeCurrentItem}
          expanded={badgeExpanded}
          visible={!menuOpen && !(bubblePayload !== null && bubbleVisible) && badgeItems.length > 0}
          onToggle={toggleBadgeExpanded}
        />
        <SpriteBubble
          payload={bubblePayload}
          visible={bubbleVisible}
        />
      </div>
    </div>
  );
}
