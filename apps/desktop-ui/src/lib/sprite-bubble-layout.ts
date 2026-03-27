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
export const BUBBLE_EXTRA_HEIGHT = 120;

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
export const MINI_CHAT_TRAY_TOTAL_HEIGHT = 60;

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

function getBubbleHeight(state: SpriteWindowState): number {
  if (state.miniChatOpen && state.miniChatBubbleOpen) {
    return state.miniChatBubbleExpanded ? 224 : 80;
  } else if (state.bubbleOpen) {
    return BUBBLE_EXTRA_HEIGHT;
  } else if (state.menuOpen) {
    return 60; // Upward compensation to fit the tall plugins menu popup
  }
  return 0;
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

export const BASE_PADDING_TOP = 36;

export function getSpriteWindowSize(state: SpriteWindowState) {
  const width = getMiniChatWidth(state);
  const bubbleH = getBubbleHeight(state);
  const paddingTop = bubbleH + BASE_PADDING_TOP;

  // Window height exactly bounds the Sprite box (paddingTop + 250) + bottom tray
  let height = paddingTop + SPRITE_HEIGHT;

  if (state.menuOpen) {
    height += 0; // The menu fits entirely within the sprite body + popup upward compensation
  } else if (state.miniChatOpen) {
    height += MINI_CHAT_TRAY_TOTAL_HEIGHT; // +60px allocated for the chat form
  } else {
    // Adding badge extra height (if expanded) so that it doesn't get cut off vertically
    height += peekBadgeExtraHeight(state.peekBadgeItemCount, state.peekBadgeExpanded);
  }

  // Exact compensation needed to negate upper padding growth from base BASE_PADDING_TOP
  const bubbleCompensationTop = bubbleH;

  return {
    width,
    height,
    /** How much the window grows leftward to keep the sprite centered. */
    extraLeft: Math.max(0, (width - SPRITE_WIDTH) / 2),
    positionCompensationTop: bubbleCompensationTop,
    extraTop: bubbleCompensationTop,
  };
}

export function getSpriteStagePadding(
  state: SpriteWindowState,
): SpriteStagePadding {
  const { width } = getSpriteWindowSize(state);
  const extraLeft = state.miniChatOpen ? (width - SPRITE_WIDTH) / 2 : 0;

  const bubbleH = getBubbleHeight(state);
  const paddingTop = bubbleH + BASE_PADDING_TOP;

  return {
    paddingTop,
    paddingBottom: 0,
    paddingLeft: extraLeft,
    paddingRight: extraLeft,
  };
}
