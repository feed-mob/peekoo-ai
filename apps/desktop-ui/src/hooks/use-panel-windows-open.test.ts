import { beforeEach, describe, expect, mock, test } from "bun:test";

let existingWindow: { setFocus: () => Promise<void> } | null = null;
let focusCount = 0;
let createdWindows: Array<{ label: string; options: Record<string, unknown> }> = [];
let monitorPointCalls: Array<{ x: number; y: number }> = [];

const spriteState = {
  scaleFactor: 2,
  outerPosition: { x: 2000, y: 400, logical: { x: 1000, y: 200 } },
  outerSize: { width: 320, height: 240, logical: { width: 160, height: 120 } },
};

let monitorState: { position: { x: number; y: number }; size: { width: number; height: number } } | null = {
  position: { x: 960, y: 0 },
  size: { width: 800, height: 600 },
};

class MockWebviewWindow {
  label: string;
  options: Record<string, unknown>;

  constructor(label: string, options: Record<string, unknown>) {
    this.label = label;
    this.options = options;
    createdWindows.push({ label, options });
  }

  static async getByLabel(): Promise<{ setFocus: () => Promise<void> } | null> {
    return existingWindow;
  }
}

mock.module("@tauri-apps/api/webviewWindow", () => ({
  WebviewWindow: MockWebviewWindow,
}));

mock.module("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    scaleFactor: async () => spriteState.scaleFactor,
    outerPosition: async () => ({
      x: spriteState.outerPosition.x,
      y: spriteState.outerPosition.y,
      toLogical: () => spriteState.outerPosition.logical,
    }),
    outerSize: async () => ({
      width: spriteState.outerSize.width,
      height: spriteState.outerSize.height,
      toLogical: () => spriteState.outerSize.logical,
    }),
  }),
  monitorFromPoint: async (x: number, y: number) => {
    monitorPointCalls.push({ x, y });
    if (!monitorState) {
      return null;
    }

    return {
      workArea: {
        position: { toLogical: () => monitorState.position },
        size: { toLogical: () => monitorState.size },
      },
    };
  },
}));

const { openPanelWindow } = await import("./use-panel-windows");

beforeEach(() => {
  existingWindow = null;
  focusCount = 0;
  createdWindows = [];
  monitorPointCalls = [];

  spriteState.scaleFactor = 2;
  spriteState.outerPosition = { x: 2000, y: 400, logical: { x: 1000, y: 200 } };
  spriteState.outerSize = { width: 320, height: 240, logical: { width: 160, height: 120 } };
  monitorState = {
    position: { x: 960, y: 0 },
    size: { width: 800, height: 600 },
  };
});

describe("openPanelWindow", () => {
  test("focuses an existing panel window instead of creating a new one", async () => {
    existingWindow = {
      setFocus: async () => {
        focusCount += 1;
      },
    };

    const result = await openPanelWindow("panel-chat");

    expect(result).toBe(existingWindow);
    expect(focusCount).toBe(1);
    expect(createdWindows).toHaveLength(0);
    expect(monitorPointCalls).toHaveLength(0);
  });

  test("creates a new panel window using logical monitor work area bounds", async () => {
    spriteState.outerPosition = { x: 2200, y: 400, logical: { x: 1100, y: 200 } };

    const result = await openPanelWindow("panel-chat");

    expect(result).toBeInstanceOf(MockWebviewWindow);
    expect(monitorPointCalls).toEqual([{ x: 2200, y: 400 }]);
    expect(createdWindows).toHaveLength(1);
    expect(createdWindows[0]).toEqual({
      label: "panel-chat",
      options: expect.objectContaining({
        title: "Peekoo Chat",
        width: 520,
        height: 640,
        x: 1224,
        y: 16,
      }),
    });
  });
});
