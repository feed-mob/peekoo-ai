# 2026-03-06 12:30: fix: Sprite sheet dimensions mismatch

## What changed

Fixed the sprite sheet rendering issue where the entire sprite image displayed as a tiny thumbnail instead of individual animated frames.

## Why

The `SPRITE_CONFIG` in `SpriteAnimation.tsx` had the old sprite sheet dimensions (4416x3840, 552x548 per frame) but the actual `sprite.jpg` was only 1024x890. The canvas was sampling 552x548 regions from a 1024px-wide image, causing the entire sheet to render as a tiny thumbnail.

## Root cause analysis

The sprite asset was replaced with a smaller image but `SPRITE_CONFIG` was not updated:

| File | Dimensions | Format | Frame size |
|------|-----------|--------|-----------|
| `sprite0.jpg` (old) | 4416x3840 | JPEG | 552x548 |
| `sprite.jpg` (broken) | 1024x890 | JPEG | 128x~127 (uneven) |
| `sprite2.jpg` (correct) | 1024x896 | PNG | 128x128 (clean) |

`sprite.jpg` (1024x890) doesn't divide evenly by 7 rows (890/7 = 127.14). `sprite2.jpg` (1024x896) has clean 128x128 frames (896/7 = 128 exactly).

## Files affected

- `apps/desktop-ui/public/sprite.png` — copied from `sprite2.jpg` (renamed to correct extension)
- `apps/desktop-ui/src/components/sprite/SpriteAnimation.tsx` — updated `SPRITE_CONFIG`
- `apps/desktop-ui/src/components/sprite/Sprite.tsx` — adjusted scale prop

## Details

### SPRITE_CONFIG changes

```typescript
// Before
frameWidth: 552,   // 4416 / 8
frameHeight: 548,  // 3840 / 7
imageSrc: "/sprite.jpg",

// After
frameWidth: 128,   // 1024 / 8
frameHeight: 128,  // 896 / 7
imageSrc: "/sprite.png",
```

### Scale adjustment

To maintain the same visual size (~109px display):
- Old: 552 * 0.2 = 110px
- New: 128 * 0.85 = 109px

Changed `scale` prop from `0.2` to `0.85` in `Sprite.tsx`.

### Asset notes

- `sprite2.jpg` is actually a PNG with an alpha channel (despite `.jpg` extension)
- The alpha channel is fully opaque — the background is still magenta-ish (#E104DE), not transparent
- Chroma key processing is still required to remove the magenta background
