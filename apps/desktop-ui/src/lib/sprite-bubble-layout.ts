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

interface SpriteWindowState {
  menuOpen: boolean;
  bubbleOpen: boolean;
}

export function getSpriteWindowSize(state: SpriteWindowState) {
  return {
    width: SPRITE_WIDTH,
    height: Math.max(
      SPRITE_WINDOW_SIZE.height,
      state.menuOpen ? SPRITE_MENU_WINDOW_SIZE.height : SPRITE_WINDOW_SIZE.height,
      state.bubbleOpen ? SPRITE_BUBBLE_WINDOW_SIZE.height : SPRITE_WINDOW_SIZE.height,
    ),
    /** How much the window grows upward (positive = window top moves up). */
    extraTop: state.bubbleOpen ? BUBBLE_EXTRA_HEIGHT : 0,
  };
}
