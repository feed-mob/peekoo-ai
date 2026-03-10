# 2026-03-10 Feat: Sprite Store Architecture & Visual Fixes

## Summary
Refactored the sprite rendering system to support a "Sprite Store" architecture using JSON manifests. This allows for multiple sprites with unique configurations (scale, frame rate, chroma key settings) to coexist. 

Also fixed a visual artifact where the default "Dark Cat" sprite had purple borders due to imperfect chroma keying.

## Key Changes

### 1. Sprite Store Architecture
- **Directory Structure**: Moved sprites from root `public/` to `public/sprites/[id]/`.
  - `public/sprites/dark-cat/`: The default black cat.
  - `public/sprites/cute-dog/`: The alternative dog sprite.
- **Manifest System**: Each sprite now has a `manifest.json` defining its properties.
  - `id`, `name`, `description`: Metadata.
  - `image`: Relative path to the sprite sheet.
  - `layout`: Columns/Rows configuration (dynamic sprite sheet support).
  - `scale`: Custom scaling factor (e.g., `0.25`).
  - `frameRate`: Custom animation speed (e.g., `6` fps).
  - `chromaKey`: Per-sprite green-screen settings.
  - `pixelArt`: Toggle for `image-rendering` (pixelated vs. smooth).

### 2. Visual Fixes (Purple Borders)
- **Problem**: The dark cat sprite had magenta/purple fringes because the chroma key algorithm wasn't aggressive enough against dark edge pixels.
- **Solution**: 
  - Implemented `stripDarkFringe` option in `chromaKey.ts`.
  - When enabled, it blindly removes pixels that match the "magenta candidate" criteria (R>G, B>G) without checking distance, which effectively strips dark purple edges on sprites that don't contain purple naturally.
  - Updated `dark-cat` manifest with `threshold: 200` and `stripDarkFringe: true`.

### 3. Rendering Quality
- **High-DPI Support**: Updated `SpriteAnimation.tsx` to respect `window.devicePixelRatio`.
  - Canvas internal resolution is now scaled to match screen density (Retina support).
  - CSS size remains logical.
  - `ctx.scale()` handles the coordinate mapping.
- **Smoothing**: Added `pixelArt` boolean to manifest. Defaults to `false` (smooth), which looks better for high-res illustrations scaled down.

## Files Modified
- `apps/desktop-ui/src/types/sprite.ts`: Added `SpriteManifest` interface.
- `apps/desktop-ui/src/components/sprite/Sprite.tsx`: Added manifest fetching logic and `activeSpriteId` state.
- `apps/desktop-ui/src/components/sprite/SpriteAnimation.tsx`: Made rendering dynamic based on props; added High-DPI support.
- `apps/desktop-ui/src/components/sprite/chromaKey.ts`: Added `stripDarkFringe` logic.
- `apps/desktop-ui/public/sprites/*`: New asset structure.
