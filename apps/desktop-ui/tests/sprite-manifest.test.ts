import { describe, expect, test } from "bun:test";
import { getActiveSpriteManifest } from "../src/components/sprite/spriteManifest";
import type { SpriteManifest } from "../src/types/sprite";

const cuteDogManifest: SpriteManifest = {
  id: "cute-dog",
  name: "Cute Dog",
  description: "Dog",
  image: "sprite.png",
  layout: { columns: 8, rows: 7 },
  scale: 0.4,
  frameRate: 6,
  chromaKey: {
    targetColor: [255, 0, 255],
    minRbOverG: 32,
    threshold: 100,
    softness: 80,
    spillSuppression: {
      enabled: true,
      threshold: 260,
      strength: 0.9,
    },
    pixelArt: false,
  },
};

describe("getActiveSpriteManifest", () => {
  test("returns the manifest when it matches the active sprite", () => {
    expect(
      getActiveSpriteManifest({ "cute-dog": cuteDogManifest }, "cute-dog"),
    ).toBe(cuteDogManifest);
  });

  test("returns null while the next sprite manifest is still loading", () => {
    expect(
      getActiveSpriteManifest({ "cute-dog": cuteDogManifest }, "dark-cat"),
    ).toBeNull();
  });

  test("returns the cached manifest for a previously loaded sprite", () => {
    const darkCatManifest: SpriteManifest = {
      ...cuteDogManifest,
      id: "dark-cat",
      name: "Dark Cat",
      scale: 0.15,
    };

    expect(
      getActiveSpriteManifest(
        {
          "cute-dog": cuteDogManifest,
          "dark-cat": darkCatManifest,
        },
        "dark-cat",
      ),
    ).toBe(darkCatManifest);
  });
});
