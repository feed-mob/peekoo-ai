export interface PluginsPopupPositionInput {
  /** Width of the popup in px. */
  popupWidth: number;
  /** Horizontal offset of the plugins button from the sprite center (from layout). */
  buttonOffsetX: number;
  /** Minimum distance (px) the tail must stay from the popup edge. */
  tailPadding: number;
}

export interface PluginsPopupPositionResult {
  /**
   * Horizontal offset of the tail tip from the popup's left edge (px).
   * Clamped so the tail stays within the popup body.
   */
  tailOffsetX: number;
}

/**
 * Compute the tail position for a sprite-centered plugins popup.
 *
 * The popup is always horizontally centered above the sprite (via CSS),
 * so we only need to figure out where the tail arrow should sit so it
 * points toward the plugins button.
 */
export function computePluginsPopupPosition({
  popupWidth,
  buttonOffsetX,
  tailPadding,
}: PluginsPopupPositionInput): PluginsPopupPositionResult {
  // The popup is centered, so its own center is at popupWidth / 2.
  // The button sits buttonOffsetX px from the sprite center, which is
  // also the popup center.
  const idealTailX = popupWidth / 2 + buttonOffsetX;

  // Clamp so the tail stays inside the popup body
  const tailOffsetX = Math.min(
    Math.max(idealTailX, tailPadding),
    popupWidth - tailPadding,
  );

  return { tailOffsetX };
}
