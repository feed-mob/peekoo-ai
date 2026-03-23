/** Width/height of the sprite window in its normal (idle) state. */
export const SPRITE_WIDTH = 200;
export const SPRITE_HEIGHT = 250;

export const SPRITE_WINDOW_SIZE = {
  width: SPRITE_WIDTH,
  height: SPRITE_HEIGHT,
} as const;

/** Extra height added below the sprite when the action menu is open. */
export const MENU_EXTRA_HEIGHT = 100;

export const SPRITE_MENU_WINDOW_SIZE = {
  width: SPRITE_WIDTH,
  height: SPRITE_HEIGHT + MENU_EXTRA_HEIGHT,
} as const;

/** Extra height added above the sprite when a speech bubble is visible. */
export const BUBBLE_EXTRA_HEIGHT = 90;

/** Width of the sprite window when mini chat input is visible. */
export const MINI_CHAT_OPEN_WIDTH = 280;

/** Extra height added below the sprite when mini chat input is visible. */
export const MINI_CHAT_EXTRA_HEIGHT = 100;

/**
 * Combined height when mini chat is open and also showing a reply bubble.
 * The sprite window already shifts upward for the bubble, so we only add the
 * extra bottom room needed to keep the input row visible.
 */
export const MINI_CHAT_WITH_BUBBLE_HEIGHT = 430;
export const MINI_CHAT_EXPANDED_BUBBLE_HEIGHT = 580;
export const MINI_CHAT_EXPANDED_BUBBLE_EXTRA_TOP = 200;
export const MINI_CHAT_EXPANDED_BUBBLE_WIDTH = 320;

/** Height of the mini chat input tray including vertical gap for absolute positioning. */
export const MINI_CHAT_TRAY_TOTAL_HEIGHT = 86;

export const SPRITE_BUBBLE_WINDOW_SIZE = {
  width: SPRITE_WIDTH,
  height: SPRITE_HEIGHT + BUBBLE_EXTRA_HEIGHT,
} as const;

/** Width of the speech bubble. */
export const BUBBLE_WIDTH = 180;

/** Height of a single collapsed peek badge. */
export const PEEK_BADGE_HEIGHT = 44;

/** Vertical padding around the badge area. */
export const PEEK_BADGE_PADDING = 8;

/** Height of each row in the expanded badge list. */
export const PEEK_BADGE_ROW_HEIGHT = 28;

/** Total vertical padding inside the expanded badge container. */
export const PEEK_BADGE_EXPANDED_VERTICAL_PADDING = 16;

export function peekBadgeExtraHeight(
  itemCount: number,
  expanded: boolean,
): number {
  if (itemCount === 0) return 0;
  if (!expanded) return PEEK_BADGE_HEIGHT + PEEK_BADGE_PADDING;
  return (
    itemCount * PEEK_BADGE_ROW_HEIGHT +
    PEEK_BADGE_PADDING +
    PEEK_BADGE_EXPANDED_VERTICAL_PADDING
  );
}

interface SpriteWindowState {
  menuOpen: boolean;
  bubbleOpen: boolean;
  peekBadgeItemCount: number;
  peekBadgeExpanded: boolean;
  miniChatOpen?: boolean;
  miniChatBubbleOpen?: boolean;
  miniChatBubbleExpanded?: boolean;
}

interface SpriteStagePadding {
  paddingTop: number;
  paddingBottom: number;
  paddingLeft: number;
  paddingRight: number;
}

function getMiniChatWidth(state: SpriteWindowState): number {
  if (
    state.miniChatOpen &&
    state.miniChatBubbleOpen &&
    state.miniChatBubbleExpanded
  ) {
    return MINI_CHAT_EXPANDED_BUBBLE_WIDTH;
  }
  if (state.miniChatOpen) {
    return MINI_CHAT_OPEN_WIDTH;
  }
  return SPRITE_WIDTH;
}

export function getSpriteWindowSize(state: SpriteWindowState) {
  const badgeExtra =
    state.bubbleOpen || state.menuOpen || state.miniChatOpen
      ? 0
      : peekBadgeExtraHeight(state.peekBadgeItemCount, state.peekBadgeExpanded);
  const miniChatExtra = state.miniChatOpen ? MINI_CHAT_EXTRA_HEIGHT : 0;
  const miniChatBubbleHeight =
    state.miniChatOpen && state.miniChatBubbleOpen
      ? state.miniChatBubbleExpanded
        ? MINI_CHAT_EXPANDED_BUBBLE_HEIGHT
        : MINI_CHAT_WITH_BUBBLE_HEIGHT
      : SPRITE_WINDOW_SIZE.height;
  const width = getMiniChatWidth(state);

  return {
    width,
    height: Math.max(
      SPRITE_WINDOW_SIZE.height,
      state.menuOpen
        ? SPRITE_MENU_WINDOW_SIZE.height
        : SPRITE_WINDOW_SIZE.height,
      state.bubbleOpen
        ? SPRITE_BUBBLE_WINDOW_SIZE.height
        : SPRITE_WINDOW_SIZE.height,
      SPRITE_WINDOW_SIZE.height + badgeExtra,
      SPRITE_WINDOW_SIZE.height + miniChatExtra,
      miniChatBubbleHeight,
    ),
    /** How much the window grows leftward to keep the sprite centered. */
    extraLeft: Math.max(0, (width - SPRITE_WIDTH) / 2),
    /** How much the window grows upward (positive = window top moves up). */
    extraTop:
      state.miniChatBubbleOpen && state.miniChatBubbleExpanded
        ? MINI_CHAT_EXPANDED_BUBBLE_EXTRA_TOP
        : state.bubbleOpen || state.miniChatBubbleOpen
          ? BUBBLE_EXTRA_HEIGHT
          : badgeExtra,
  };
}

export function getSpriteStagePadding(
  state: SpriteWindowState,
): SpriteStagePadding {
  const { width, height } = getSpriteWindowSize(state);
  const extraLeft = state.miniChatOpen ? (width - SPRITE_WIDTH) / 2 : 0;

  // Bottom section: Mini chat input tray height is ~54px (p-1.5 + h-7 + mt-2 + h-7).
  // MINI_CHAT_TRAY_TOTAL_HEIGHT adds extra gap and accounts for absolute positioning.
  const trayHeight = state.miniChatOpen ? MINI_CHAT_TRAY_TOTAL_HEIGHT : 12;

  // Top section: Reply bubble height.
  // Compact is ~80px (header + 2 short lines), Expanded is ~224px (header + 156px scrollable).
  let bubbleHeight = 12;
  if (state.miniChatOpen && state.miniChatBubbleOpen) {
    bubbleHeight = state.miniChatBubbleExpanded ? 224 : 80;
  } else if (state.bubbleOpen) {
    bubbleHeight = BUBBLE_EXTRA_HEIGHT;
  }

  // Calculate available space for sprite (SPRITE_HEIGHT = 250)
  // We want to center the sprite in the gap between bubbleHeight and trayHeight.
  const totalContentHeight = bubbleHeight + SPRITE_HEIGHT + trayHeight;
  const verticalGap = Math.max(0, height - totalContentHeight);

  // Distribute half of the gap to top and half to bottom,
  // added to their respective base heights.
  const paddingTop = bubbleHeight + verticalGap / 2;
  const paddingBottom = trayHeight + verticalGap / 2;

  return {
    paddingTop,
    paddingBottom,
    paddingLeft: extraLeft,
    paddingRight: extraLeft,
  };
}
