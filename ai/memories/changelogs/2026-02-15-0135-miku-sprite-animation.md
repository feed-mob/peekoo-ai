# 2026-02-15: Implement Hatsune Miku Sprite Animation

## Summary
Implemented sprite-based animation using the downloaded Hatsune Miku sprite sheet to replace emoji-based pet display. The sprite sheet (4416×3840, 8×7 grid, 56 frames) now renders via canvas-based animation component.

## Changes

### Frontend Components

#### New Files
- `apps/desktop-ui/src/components/SpriteAnimation.tsx` - Canvas-based sprite animation component that renders 56 frames from 8×7 grid at 8fps with configurable scale and frame rate
- `apps/desktop-ui/src/components/TestSprite.tsx` - Test component for manually testing all 7 animation states
- `apps/desktop-ui/src/TestPage.tsx` - Test page wrapper

#### Modified Files
- `apps/desktop-ui/src/components/Sprite.tsx` (renamed from Pet.tsx)
  - Replaced emoji display with SpriteAnimation component
  - Maps mood states (happy, sad, excited, thinking, idle, tired, surprised) to animation rows
  - Uses SpriteAnimation internally with 0.3 scale (~165×164px display)

- `apps/desktop-ui/src/App.tsx`
  - Changed imports from `Pet` to `Sprite`
  - Renamed `petState` to `spriteState`
  - Updated Tauri invoke call from `get_pet_state` to `get_sprite_state`
  - Updated default message to reference "sprite" instead of "pet"

- `apps/desktop-ui/src/components/Pomodoro.tsx`
  - Removed unused `React` and `useCallback` imports
  - Fixed `NodeJS.Timeout` type to `number` for browser compatibility

### Backend

#### Modified Files
- `apps/desktop-tauri/src-tauri/src/lib.rs`
  - Renamed `get_pet_state` command to `get_sprite_state`
  - Updated invoke handler registration
  - Updated default animation state to return "happy"

### Assets
- `apps/desktop-ui/public/sprite.jpg` - Added 11MB Hatsune Miku sprite sheet (4416×3840 JPEG, 8 columns × 7 rows)

## Technical Implementation

### Sprite Configuration
```typescript
const SPRITE_CONFIG = {
  columns: 8,
  rows: 7,
  frameWidth: 552,   // 4416 / 8
  frameHeight: 548, // 3840 / 7
  imageSrc: '/sprite.jpg',
};
```

### Animation Mapping
| Row | Animation | Description |
|-----|-----------|-------------|
| 0   | idle      | Breathing + blinking |
| 1   | happy     | Happy/love |
| 2   | excited   | Excited/celebrate |
| 3   | sleepy    | Sleepy/snoring (eyes closed) |
| 4   | working   | Working |
| 5   | angry     | Angry/surprised/shy |
| 6   | dragging  | Dragging |

### Mood to Animation Mapping
| Mood      | Animation |
|-----------|-----------|
| happy     | happy     |
| sad       | angry     |
| excited   | excited   |
| thinking  | working   |
| idle      | idle      |
| tired     | sleepy    |
| surprised | angry     |

## Key Fixes

1. **Animation Loop Bug** - Fixed `useEffect` dependency array that was causing animation loop to restart on every render. Removed `onFrameChange` from dependencies and used optional chaining `onFrameChange?.()`.

2. **TypeScript Errors** - Fixed multiple TS errors:
   - Removed unused `React` imports
   - Fixed duplicate `imageRendering` property in style object
   - Changed `NodeJS.Timeout` to `number` for browser compatibility

3. **Naming Consistency** - Renamed all references from "pet" to "sprite" throughout codebase for clarity.

## Testing

Build succeeds with no TypeScript errors:
```bash
cd apps/desktop-ui
bun run build
```

## Usage

Run the desktop app:
```bash
just dev
```

The sprite should display animated Hatsune Miku (初音未来) instead of emoji.

## Debugging

Console logging added to `SpriteAnimation.tsx`:
- `[SpriteAnimation] Attempting to load image from: /sprite.jpg`
- `[SpriteAnimation] Image loaded successfully`
- `[SpriteAnimation] Canvas size set to: {...}`
- `[SpriteAnimation] Animation changed to: {animation}`
- `[SpriteAnimation] Starting animation loop`

## Future Enhancements

- Consider creating PNG version with transparent background instead of JPEG with magenta background
- Add drag-and-drop support for dragging animation state
- Implement frame-by-frame playback control
- Add sprite sheet switching support for multiple characters
