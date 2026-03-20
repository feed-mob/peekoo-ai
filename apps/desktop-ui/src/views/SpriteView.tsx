import { useState, useCallback, useEffect, useMemo, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Sprite } from "@/components/sprite/Sprite";
import { SpriteActionMenu } from "@/components/sprite/SpriteActionMenu";
import { SpriteBubble } from "@/components/sprite/SpriteBubble";
import { SpritePeekBadge } from "@/components/sprite/SpritePeekBadge";
import { SpriteMiniChat } from "@/components/sprite/SpriteMiniChat";
import { SpriteMiniChatBubble } from "@/components/sprite/SpriteMiniChatBubble";
import { usePeekBadge } from "@/hooks/use-peek-badge";
import { useSpriteBubble } from "@/hooks/use-sprite-bubble";
import {
  getSpriteStagePadding,
  getSpriteWindowSize,
} from "@/lib/sprite-bubble-layout";
import { useSpriteState } from "@/hooks/use-sprite-state";
import { usePanelWindows } from "@/hooks/use-panel-windows";
import { useSpriteReactions } from "@/hooks/use-sprite-reactions";
import { useIdleStateManager } from "@/hooks/use-idle-state-manager";
import {
  getLatestMiniChatMessage,
  getMiniChatReplyDisplayMode,
  getMiniChatVisibleMessage,
  useChatSession,
} from "@/features/chat/chat-session";
import {
  SPRITE_BUBBLE_DURATION_MS,
  SPRITE_BUBBLE_EVENT,
  SpriteBubblePayloadSchema,
} from "@/types/sprite-bubble";
import type { PanelLabel } from "@/types/window";
import type { AnimationType, SpriteState } from "@/types/sprite";

// Duration (ms) a reaction-triggered mood override stays active before reverting
const MOOD_OVERRIDE_DURATION_MS = 3000;
const DRAG_THRESHOLD_PX = 8;

export async function openSettingsPanelFromTray(
  openPanel: (label: PanelLabel) => Promise<void>,
) {
  await openPanel("panel-settings");
}

export async function openAboutPanelFromTray(
  openPanel: (label: PanelLabel) => Promise<void>,
) {
  await openPanel("panel-about");
}

export default function SpriteView() {
  const spriteState = useSpriteState();
  const {
    payload: bubblePayload,
    visible: bubbleVisible,
    showBubble,
    clearBubble,
  } = useSpriteBubble();
  const {
    items: badgeItems,
    currentItem: badgeCurrentItem,
    expanded: badgeExpanded,
    toggleExpanded: toggleBadgeExpanded,
    collapse: collapseBadge,
  } = usePeekBadge();
  const { panels, pluginPanels, installedPlugins, openPanel, togglePanel } =
    usePanelWindows();
  const {
    messages: chatMessages,
    isTyping: chatIsTyping,
    sendMessage,
  } = useChatSession();
  const [menuOpen, setMenuOpen] = useState(false);
  const [miniChatOpen, setMiniChatOpen] = useState(false);
  const miniChatOpenRef = useRef(false);
  const [miniChatActiveReplyId, setMiniChatActiveReplyId] = useState<
    string | null
  >(null);
  const [miniChatAwaitingReply, setMiniChatAwaitingReply] = useState(false);
  const [moodOverride, setMoodOverride] = useState<string | null>(null);
  const [dragAnimation, setDragAnimation] = useState<AnimationType | null>(
    null,
  );
  const moodResetTimerRef = useRef<number | null>(null);
  const interactionRootRef = useRef<HTMLDivElement | null>(null);
  const dragStateRef = useRef<{
    startX: number;
    startY: number;
    dragging: boolean;
  } | null>(null);
  useEffect(() => {
    miniChatOpenRef.current = miniChatOpen;
  }, [miniChatOpen]);

  const latestMiniChatMessage = getMiniChatVisibleMessage({
    messages: chatMessages,
    activeReplyId: miniChatActiveReplyId,
  });
  const miniChatReplyDisplayMode = getMiniChatReplyDisplayMode(
    latestMiniChatMessage,
  );
  const miniChatBubbleVisible =
    miniChatOpen && (chatIsTyping || latestMiniChatMessage !== null);
  const spriteWindowState = useMemo(
    () => ({
      menuOpen,
      bubbleOpen: bubblePayload !== null && bubbleVisible && !miniChatOpen,
      peekBadgeItemCount: badgeItems.length,
      peekBadgeExpanded: badgeExpanded,
      miniChatOpen,
      miniChatBubbleOpen: miniChatBubbleVisible,
      miniChatBubbleExpanded:
        !chatIsTyping && miniChatReplyDisplayMode === "expanded",
    }),
    [
      badgeExpanded,
      badgeItems.length,
      bubblePayload,
      bubbleVisible,
      chatIsTyping,
      menuOpen,
      miniChatBubbleVisible,
      miniChatOpen,
      miniChatReplyDisplayMode,
    ],
  );
  const stagePadding = getSpriteStagePadding(spriteWindowState);

  useEffect(() => {
    if (chatIsTyping || !miniChatAwaitingReply) {
      return;
    }

    const latestReply = getLatestMiniChatMessage(chatMessages);
    setMiniChatActiveReplyId(latestReply?.id ?? null);
    setMiniChatAwaitingReply(false);
  }, [chatIsTyping, chatMessages, miniChatAwaitingReply]);

  // Idle state manager for random state transitions
  const { randomState, resetIdleTimer } = useIdleStateManager({
    enabled: true,
    isUserInteracting: menuOpen || dragAnimation !== null,
    hasActiveNotification:
      moodOverride === "reminder" || (bubblePayload !== null && bubbleVisible),
  });

  const clearMoodResetTimer = useCallback(() => {
    if (moodResetTimerRef.current !== null) {
      window.clearTimeout(moodResetTimerRef.current);
      moodResetTimerRef.current = null;
    }
  }, []);

  const handleMoodChange = useCallback(
    (mood: string, sticky: boolean) => {
      clearMoodResetTimer();
      setMoodOverride(mood);

      if (!sticky) {
        moodResetTimerRef.current = window.setTimeout(() => {
          setMoodOverride(null);
          moodResetTimerRef.current = null;
        }, MOOD_OVERRIDE_DURATION_MS);
      }
    },
    [clearMoodResetTimer],
  );

  useEffect(() => {
    return () => {
      clearMoodResetTimer();
      clearBubble();
    };
  }, [clearBubble, clearMoodResetTimer]);

  // Track the previous extraTop so we can compute the delta for position adjustment.
  const prevExtraTopRef = useRef(0);
  const prevExtraLeftRef = useRef(0);

  // Auto-expand/shrink the main window when bubble visibility or menu state changes.
  // We invoke a Rust command so the backend can keep size constraints synchronized
  // with the current target size for reliable constrained resizing across platforms.
  useEffect(() => {
    const nextSize = getSpriteWindowSize(spriteWindowState);
    const deltaTop = nextSize.extraTop - prevExtraTopRef.current;
    const deltaLeft = nextSize.extraLeft - prevExtraLeftRef.current;
    prevExtraTopRef.current = nextSize.extraTop;
    prevExtraLeftRef.current = nextSize.extraLeft;
    void invoke("resize_sprite_window", {
      width: nextSize.width,
      height: nextSize.height,
      deltaLeft,
      deltaTop,
    });
  }, [spriteWindowState]);

  useEffect(() => {
    const unlisten = listen(SPRITE_BUBBLE_EVENT, (event) => {
      const parsed = SpriteBubblePayloadSchema.safeParse(event.payload);
      if (!parsed.success) {
        return;
      }

      collapseBadge();
      setMiniChatOpen(false);
      setMiniChatActiveReplyId(null);
      setMiniChatAwaitingReply(false);
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

  useEffect(() => {
    if (!miniChatOpen) {
      return;
    }

    const handlePointerDown = (event: MouseEvent) => {
      const target = event.target;
      if (!(target instanceof Node)) {
        return;
      }

      if (!interactionRootRef.current?.contains(target)) {
        setMiniChatOpen(false);
        setMiniChatActiveReplyId(null);
        setMiniChatAwaitingReply(false);
      }
    };

    window.addEventListener("mousedown", handlePointerDown);
    return () => {
      window.removeEventListener("mousedown", handlePointerDown);
    };
  }, [miniChatOpen]);

  // Open settings panel when the tray menu "Settings" item is clicked
  useEffect(() => {
    const unlisten = listen("open-settings", () => {
      void openSettingsPanelFromTray(openPanel);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [openPanel]);

  // Open about panel when the tray menu "About Peekoo" item is clicked
  useEffect(() => {
    const unlisten = listen("open-about", () => {
      void openAboutPanelFromTray(openPanel);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [openPanel]);

  // Determine effective sprite state with priority:
  // 1. moodOverride (reactions, reminders) - highest priority
  // 2. randomState (idle state manager) - low priority
  // 3. spriteState (default) - fallback
  const effectiveSpriteState: SpriteState = moodOverride
    ? { ...spriteState, mood: moodOverride }
    : randomState
      ? { ...spriteState, mood: randomState }
      : spriteState;

  const startWindowDrag = useCallback(async () => {
    setDragAnimation("dragging");

    try {
      await new Promise((resolve) =>
        requestAnimationFrame(() => resolve(null)),
      );
      await getCurrentWindow().startDragging();
    } catch (error) {
      console.error("Failed to start dragging sprite window", error);
      setDragAnimation(null);
    }
  }, []);

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

  const handleMouseDown = useCallback(
    async (e: React.MouseEvent) => {
      if (e.button !== 0) return;
      e.stopPropagation();

      resetIdleTimer();

      dragStateRef.current = {
        startX: e.clientX,
        startY: e.clientY,
        dragging: false,
      };

      const handleMouseMove = (event: MouseEvent) => {
        const dragState = dragStateRef.current;
        if (!dragState || dragState.dragging) {
          return;
        }

        const deltaX = event.clientX - dragState.startX;
        const deltaY = event.clientY - dragState.startY;
        if (Math.hypot(deltaX, deltaY) < DRAG_THRESHOLD_PX) {
          return;
        }

        dragState.dragging = true;
        void startWindowDrag();
      };

      const handleMouseUp = () => {
        const dragState = dragStateRef.current;
        dragStateRef.current = null;
        window.removeEventListener("mousemove", handleMouseMove);
        window.removeEventListener("mouseup", handleMouseUp);

        if (!dragState?.dragging) {
          collapseBadge();
          setMenuOpen(false);
          setMiniChatOpen((prev) => !prev);
          if (miniChatOpenRef.current) {
            setMiniChatActiveReplyId(null);
            setMiniChatAwaitingReply(false);
          }
        }
      };

      window.addEventListener("mousemove", handleMouseMove);
      window.addEventListener("mouseup", handleMouseUp);
    },
    [collapseBadge, resetIdleTimer, startWindowDrag],
  );

  // Right click: toggle menu
  const handleContextMenu = useCallback(
    async (e: React.MouseEvent) => {
      e.preventDefault();

      // Reset idle timer on user interaction
      resetIdleTimer();

      if (menuOpen) {
        setMenuOpen(false);
      } else {
        setMiniChatOpen(false);
        setMiniChatActiveReplyId(null);
        setMiniChatAwaitingReply(false);
        collapseBadge();
        setMenuOpen(true);
      }
    },
    [collapseBadge, menuOpen, resetIdleTimer],
  );

  const handleTogglePanel = useCallback(
    async (label: Parameters<typeof togglePanel>[0]) => {
      // Reset idle timer on user interaction
      resetIdleTimer();

      await togglePanel(label);
      setMenuOpen(false);
      setMiniChatOpen(false);
      setMiniChatActiveReplyId(null);
      setMiniChatAwaitingReply(false);
    },
    [togglePanel, resetIdleTimer],
  );

  const handleMiniChatSubmit = useCallback(
    async (message: string) => {
      clearBubble();
      setMiniChatActiveReplyId(null);
      setMiniChatAwaitingReply(true);
      const didSend = await sendMessage(message);
      if (!didSend) {
        setMiniChatAwaitingReply(false);
        const latestReply = getLatestMiniChatMessage(chatMessages);
        setMiniChatActiveReplyId(latestReply?.id ?? null);
        return false;
      }

      return true;
    },
    [chatMessages, clearBubble, sendMessage],
  );

  const handleOpenFullChat = useCallback(async () => {
    await openPanel("panel-chat");
    setMiniChatOpen(false);
    setMiniChatActiveReplyId(null);
    setMiniChatAwaitingReply(false);
    setMenuOpen(false);
  }, [openPanel]);

  const handleCloseMiniChat = useCallback(() => {
    setMiniChatOpen(false);
    setMiniChatActiveReplyId(null);
    setMiniChatAwaitingReply(false);
  }, []);

  return (
    <div className="w-full h-full bg-transparent">
      <div ref={interactionRootRef} className="relative w-full h-full">
        <div
          style={{
            paddingTop: `${stagePadding.paddingTop}px`,
            paddingBottom: `${stagePadding.paddingBottom}px`,
            paddingLeft: `${stagePadding.paddingLeft}px`,
            paddingRight: `${stagePadding.paddingRight}px`,
          }}
          className="flex h-full w-full items-center justify-center"
        >
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
          visible={
            !miniChatOpen &&
            !menuOpen &&
            !(bubblePayload !== null && bubbleVisible) &&
            badgeItems.length > 0
          }
          onToggle={toggleBadgeExpanded}
        />
        <SpriteMiniChatBubble
          message={latestMiniChatMessage}
          visible={miniChatBubbleVisible}
          thinking={chatIsTyping}
          displayMode={chatIsTyping ? "compact" : miniChatReplyDisplayMode}
        />
        <SpriteBubble
          payload={bubblePayload}
          visible={bubbleVisible && !miniChatOpen}
        />
        <SpriteMiniChat
          open={miniChatOpen}
          isTyping={chatIsTyping}
          onClose={handleCloseMiniChat}
          onOpenFullChat={handleOpenFullChat}
          onSubmit={handleMiniChatSubmit}
        />
      </div>
    </div>
  );
}
