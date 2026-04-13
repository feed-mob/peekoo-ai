## 2026-04-13 09:29: fix: stabilize sprite canvas scaling on Windows

**What changed:**
- Extracted sprite canvas sizing into `apps/desktop-ui/src/components/sprite/spriteCanvasSize.ts` so logical display size and backing canvas size are computed consistently from frame size, scale, and `devicePixelRatio`.
- Updated `apps/desktop-ui/src/components/sprite/SpriteAnimation.tsx` to remove the extra `ctx.scale(...)` call and rely on a single `ctx.setTransform(...)` draw path.
- Added `apps/desktop-ui/src/components/sprite/spriteManifest.ts` and updated `Sprite.tsx` to render only when the fetched manifest matches the current `activeSpriteId`.
- Updated `Sprite.tsx` to cache fetched manifests, prefetch known sprite manifests, and apply a short fade-in when the active sprite changes.
- Added `apps/desktop-ui/tests/sprite-canvas-size.test.ts` to lock in the intended DPR behavior for integer and fractional Windows scale factors.
- Added `apps/desktop-ui/tests/sprite-manifest.test.ts` to prevent rendering a stale manifest while the next sprite's manifest is still loading.
- Added explicit sprite image load error logging in `SpriteAnimation.tsx`.

**Why:**
- The cute dog sprite rendered incorrectly in the main transparent Tauri window on Windows while still loading correctly in the settings preview.
- The renderer was applying high-DPI scaling in two different places, which made Windows/WebView2 a likely source of inconsistent sprite sizing and placement.
- Switching from dog to cat briefly rendered the cat using the dog's manifest scale before the cat manifest fetch completed, causing a visible oversized flash.
- Even after fixing the stale-manifest flash, uncached sprite switches could briefly render blank while the next manifest loaded, so caching and prefetching smooths the transition.

**Files affected:**
- `apps/desktop-ui/src/components/sprite/SpriteAnimation.tsx`
- `apps/desktop-ui/src/components/sprite/spriteCanvasSize.ts`
- `apps/desktop-ui/src/components/sprite/spriteManifest.ts`
- `apps/desktop-ui/src/components/sprite/Sprite.tsx`
- `apps/desktop-ui/tests/sprite-canvas-size.test.ts`
- `apps/desktop-ui/tests/sprite-manifest.test.ts`
