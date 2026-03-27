# Sprite System Architecture

## Overview
Peekoo's desktop pet system uses a modular "Sprite Store" architecture. Each sprite is fully self-contained in its own directory within `apps/desktop-ui/public/sprites/`.

## Window Sizing and Interaction
The sprite runs inside the main undecorated transparent Tauri window and uses frontend layout state plus a backend resize command to keep the window aligned with the visible UI.

### Base window behavior
- Default sprite window size is `200x250`
- The main window remains configured as non-resizable by default so the sprite keeps its expected click/drag behavior on Linux/Wayland compositors
- When UI chrome appears, the frontend computes the next window bounds and invokes the backend `resize_sprite_window` command

### Auto-resize flow
1. Frontend layout helpers in `apps/desktop-ui/src/lib/sprite-bubble-layout.ts` calculate:
   - target `width`
   - target `height`
   - `extraLeft` to preserve horizontal centering
   - `extraTop` to preserve vertical anchoring when bubbles expand upward
2. `SpriteView` watches sprite UI state changes and calls `resize_sprite_window`
3. The Tauri command temporarily enables resizing, applies tight min/max constraints, adjusts position, resizes the window, then restores the window to non-resizable

### Why this matters
This constrained-resize approach was added because permanently making the main transparent sprite window resizable caused broken click behavior on Linux/Wayland in practice. Temporarily toggling resizability during programmatic resize preserves sprite interaction while still allowing automatic expansion and shrink.

### Related changes
- Mini chat open state now widens and heightens the sprite window
- Expanded mini chat reading mode uses a wider bubble and a larger window width
- Panel windows remain separately resizable via explicit resize handles in `PanelShell`

### Relevant files
- `apps/desktop-ui/src/views/SpriteView.tsx`
- `apps/desktop-ui/src/lib/sprite-bubble-layout.ts`
- `apps/desktop-ui/src/lib/sprite-bubble-layout.test.ts`
- `apps/desktop-ui/src/components/sprite/SpriteMiniChatBubble.tsx`
- `apps/desktop-ui/src/components/panels/PanelShell.tsx`
- `apps/desktop-tauri/src-tauri/src/lib.rs`
- `apps/desktop-tauri/src-tauri/tauri.conf.json`
- `apps/desktop-tauri/src-tauri/capabilities/default.json`
- `ai/memories/changelogs/202603200347-fix-sprite-window-constrained-resize.md`

## Directory Structure
```text
apps/desktop-ui/public/sprites/
└── [sprite-id]/                <-- Unique ID (e.g., 'dark-cat')
    ├── manifest.json           <-- Configuration file
    └── sprite.png              <-- The sprite sheet image
```

## Manifest JSON (`manifest.json`)
Every sprite must include a `manifest.json` file defining its properties. This allows different sprites to have completely different sizes, frame rates, and chroma key settings.

```json
{
  "id": "dark-cat",
  "name": "Dark Cat",
  "description": "Default dark-themed AI pet.",
  "image": "sprite.png",       // Path relative to the manifest
  "layout": {
    "columns": 8,              // Sprite sheet columns
    "rows": 7                  // Sprite sheet rows
  },
  "scale": 0.25,               // Render scale (default: 0.40)
  "frameRate": 6,              // Animation speed (default: 8)
  "chromaKey": {
    "targetColor": [255, 0, 255], // RGB key color
    "minRbOverG": 5,           // Min difference between R/B and G to be considered key
    "threshold": 200,          // Distance threshold for transparency
    "softness": 20,            // Fade-out distance beyond threshold
    "spillSuppression": {
      "enabled": true,
      "threshold": 300,        // Distance threshold for desaturation
      "strength": 1.0          // How strongly to remove color cast
    },
    "stripDarkFringe": true,   // Aggressively remove any candidate pixel (useful for dark sprites against bright keys)
    "pixelArt": false          // Use 'pixelated' or 'auto' (smooth) rendering
  }
}
```

## Key Configuration Tips

### Fixing Purple/Green Borders
If a sprite has ugly colored borders from the background removal:
1.  **Increase Threshold**: Be careful not to eat into the character itself.
2.  **Use `stripDarkFringe: true`**: Extremely effective for dark characters on bright backgrounds (magenta/green). It removes *any* pixel that leans towards the key color, ignoring brightness. Safe only if the character contains none of the key color's hues.
3.  **Spill Suppression**: Increase `strength` to `1.0` and tweak `threshold` to convert remaining colored fringes to grayscale/black.

### Scaling Quality
- **`pixelArt: true`**: Best for retro, low-res sprites. Uses `image-rendering: pixelated`.
- **`pixelArt: false`**: Best for high-res illustrations or downscaled assets. Uses browser smoothing for a painted look.
- **High-DPI**: The rendering engine automatically scales the internal canvas resolution by `window.devicePixelRatio` for crisp rendering on Retina displays.

## Adding New Sprites
1.  Create a folder: `apps/desktop-ui/public/sprites/[new-id]`.
2.  Add your sprite sheet image.
3.  Create `manifest.json` based on the example above.
4.  Currently, switching sprites requires changing the `activeSpriteId` state in `apps/desktop-ui/src/components/sprite/Sprite.tsx`. (Future work: Build a UI for selecting sprites).
