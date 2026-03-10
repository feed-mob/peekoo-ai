# Sprite System Architecture

## Overview
Peekoo's desktop pet system uses a modular "Sprite Store" architecture. Each sprite is fully self-contained in its own directory within `apps/desktop-ui/public/sprites/`.

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
