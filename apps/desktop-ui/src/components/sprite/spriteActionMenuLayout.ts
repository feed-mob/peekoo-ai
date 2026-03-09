import type { PanelLabel } from "@/types/window";

export interface SpriteActionMenuItem {
  label: PanelLabel;
  name: string;
  x: number;
  y: number;
}

interface MenuItemDefinition {
  label: PanelLabel;
  name: string;
}

/** Vertical offset below the sprite center where the row of buttons sits. */
const ROW_Y = 72;

/** Horizontal gap between button centers. */
const ITEM_SPACING = 52;

const MENU_ITEM_DEFINITIONS: ReadonlyArray<MenuItemDefinition> = [
  { label: "panel-chat", name: "Chat" },
  { label: "panel-tasks", name: "Tasks" },
  { label: "panel-pomodoro", name: "Pomodoro" },
  { label: "panel-plugins", name: "Plugins" },
] as const;

/**
 * Lay out items in a centered horizontal row.
 *
 * For N items the total span is `(N - 1) * ITEM_SPACING`.
 * The first item sits at `-span / 2` and each subsequent item
 * is offset by `ITEM_SPACING`.
 */
function horizontalRow(
  items: ReadonlyArray<MenuItemDefinition>,
): SpriteActionMenuItem[] {
  const span = (items.length - 1) * ITEM_SPACING;
  const startX = -span / 2;

  return items.map((item, index) => ({
    label: item.label,
    name: item.name,
    x: startX + index * ITEM_SPACING,
    y: ROW_Y,
  }));
}

export function getSpriteActionMenuItems(): SpriteActionMenuItem[] {
  return horizontalRow(MENU_ITEM_DEFINITIONS);
}
