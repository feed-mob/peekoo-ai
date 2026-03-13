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

export function peekBadgeExtraHeight(itemCount: number, expanded: boolean): number {
  if (itemCount === 0) return 0;
  if (!expanded) return PEEK_BADGE_HEIGHT + PEEK_BADGE_PADDING;
  return itemCount * PEEK_BADGE_ROW_HEIGHT + PEEK_BADGE_PADDING + PEEK_BADGE_EXPANDED_VERTICAL_PADDING;
}

interface SpriteWindowState {
  menuOpen: boolean;
  bubbleOpen: boolean;
  peekBadgeItemCount: number;
  peekBadgeExpanded: boolean;
}

export function getSpriteWindowSize(state: SpriteWindowState) {
  const badgeExtra =
    state.bubbleOpen || state.menuOpen
      ? 0
      : peekBadgeExtraHeight(state.peekBadgeItemCount, state.peekBadgeExpanded);

  return {
    width: SPRITE_WIDTH,
    height: Math.max(
      SPRITE_WINDOW_SIZE.height,
      state.menuOpen ? SPRITE_MENU_WINDOW_SIZE.height : SPRITE_WINDOW_SIZE.height,
      state.bubbleOpen ? SPRITE_BUBBLE_WINDOW_SIZE.height : SPRITE_WINDOW_SIZE.height,
      SPRITE_WINDOW_SIZE.height + badgeExtra,
    ),
    /** How much the window grows upward (positive = window top moves up). */
    extraTop: state.bubbleOpen
      ? BUBBLE_EXTRA_HEIGHT
      : badgeExtra,
  };
}
