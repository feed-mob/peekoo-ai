# 2026-02-15 02:29: Sprite Chroma-Key Background Removal and Halo Reduction

## Summary
Implemented runtime chroma-key preprocessing for the JPEG sprite sheet so the magenta background is removed at render time, then tuned spill suppression to reduce pink edge halo artifacts.

## Why
- The sprite source is `sprite.jpg` (opaque JPEG, no alpha), so magenta background pixels were visible even with a transparent app window.
- Needed an immediate fix without waiting for a re-exported transparent PNG/WebP asset.

## Changes

### 1) New Chroma-Key Utility
- Added `apps/desktop-ui/src/components/sprite/chromaKey.ts`
- Introduced strict typed options:
  - `RgbColor`
  - `SpillSuppressionOptions`
  - `ChromaKeyOptions`
- Added helpers and pipeline:
  - candidate detection for magenta-like pixels
  - distance-based keying with soft feathered alpha
  - spill suppression to neutralize magenta edge tint
  - `applyChromaKeyToImageData(...)`
  - `buildKeyedSpriteSheet(...)`

### 2) SpriteAnimation Integration
- Updated `apps/desktop-ui/src/components/sprite/SpriteAnimation.tsx`
- Added prop: `chromaKey?: false | Partial<ChromaKeyOptions>`
- Switched from raw image source to preprocessed keyed source:
  - keying happens once at image load
  - animation loop remains lightweight (`clearRect` + `drawImage`)
- Set `ctx.imageSmoothingEnabled = false` for crisp pixel rendering.

### 3) Halo Reduction Tuning
- Updated `apps/desktop-ui/src/components/sprite/Sprite.tsx` with stronger per-sprite key settings.
- Updated spill suppression behavior in `chromaKey.ts` to use magenta-bias weighting (better on opaque fringe pixels).

## Current Tuned Parameters
- `targetColor`: `[255, 0, 255]`
- `minRbOverG`: `38`
- `threshold`: `84`
- `softness`: `64`
- `spillSuppression.threshold`: `230`
- `spillSuppression.strength`: `0.78`

## Validation
- `bunx tsc --noEmit` passed
- `bun run build` passed

## Follow-up
- Long-term best quality path remains migrating to a transparent sprite asset (`sprite.png` or `sprite.webp`) and disabling runtime chroma key.
