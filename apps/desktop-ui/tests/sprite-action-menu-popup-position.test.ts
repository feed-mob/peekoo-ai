import { describe, expect, test } from "bun:test";
import { computePluginsPopupPosition } from "../src/components/sprite/spriteActionMenuPopupPosition";

describe("computePluginsPopupPosition", () => {
  test("tail points toward button when button is to the right of center", () => {
    // Plugins button is 78px right of sprite center, popup is 180px wide
    const result = computePluginsPopupPosition({
      popupWidth: 180,
      buttonOffsetX: 78,
      tailPadding: 16,
    });

    // Ideal tail X = 180/2 + 78 = 168, within bounds (16..164) → clamped to 164
    expect(result.tailOffsetX).toBe(164);
  });

  test("tail points toward button when button is to the left of center", () => {
    const result = computePluginsPopupPosition({
      popupWidth: 180,
      buttonOffsetX: -78,
      tailPadding: 16,
    });

    // Ideal tail X = 180/2 + (-78) = 12, below padding → clamped to 16
    expect(result.tailOffsetX).toBe(16);
  });

  test("tail is centered when button is at sprite center", () => {
    const result = computePluginsPopupPosition({
      popupWidth: 180,
      buttonOffsetX: 0,
      tailPadding: 16,
    });

    // Ideal tail X = 180/2 + 0 = 90, within bounds
    expect(result.tailOffsetX).toBe(90);
  });

  test("tail stays within popup when button offset is very large", () => {
    const result = computePluginsPopupPosition({
      popupWidth: 180,
      buttonOffsetX: 200,
      tailPadding: 16,
    });

    // Ideal tail X = 180/2 + 200 = 290, clamped to 180 - 16 = 164
    expect(result.tailOffsetX).toBe(164);
  });

  test("tail stays within popup when button offset is very negative", () => {
    const result = computePluginsPopupPosition({
      popupWidth: 180,
      buttonOffsetX: -200,
      tailPadding: 16,
    });

    // Ideal tail X = 180/2 + (-200) = -110, clamped to 16
    expect(result.tailOffsetX).toBe(16);
  });
});
