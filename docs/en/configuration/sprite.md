# Sprite Configuration

The sprite is the character Peekoo displays on your desktop. You can use one of the built-in characters or upload your own image to create a custom companion.

<!-- Image placeholder: Sprite switcher screenshot -->

## Switch Built-in Sprites

System tray icon right-click → Settings → Appearance → Active Sprite

Two sprites are included by default:

- **Dark Cat** — the default dark-themed cat
- **Cute Dog** — a cute alternative dog character

Changes take effect immediately.

## Custom Sprites

You can upload your own character image through the settings page.

System tray icon right-click → Settings → Appearance → Custom Sprite

There are two ways to prepare your image:

### Option 1: Generate with AI

1. Click "Copy Prompt"
2. Paste it into your preferred AI image tool (ChatGPT, Midjourney, etc.)
3. Fill in the character name, style, and appearance description at the end of the prompt
4. Generate and download the image

### Option 2: Prepare your own image

If you already have an image or want to draw one yourself, create a sprite sheet following the format requirements below and upload it directly.

### Upload your image

Once you have an image ready:

1. Click "Upload Image" and select your sprite sheet file
2. Peekoo will automatically attempt to generate a manifest
3. Click "Validate Draft" to check for issues
4. Click "Save Custom Sprite" when everything looks good

You can also click "Upload Manifest" to provide your own config file, or edit the parameters directly in the JSON editor.

## Sprite Sheet Format Requirements

Peekoo uses a sprite sheet format — all animation frames arranged in a single image.

### Basic specs

- Layout: **8 columns × 7 rows** (56 frames total), each frame must be square
- Aspect ratio: approximately 8:7
- Recommended resolution: **4096 × 3584** (512 × 512 per frame); the app scales it down to 1024 × 896 automatically
- Format: PNG (recommended) or JPG

### Background color

- Background must be **pure magenta `#ff00ff`** and nothing else
- No gradients, shadows, textures, or noise
- Avoid using colors close to magenta in the character itself (shadows, highlights, outlines, etc.) — they will be keyed out along with the background
- All areas between frames and around the edges must stay pure magenta — no dark seams or separator lines

### Frame layout

- Do not draw any grid lines, borders, or separators — frames are distinguished by position only
- Leave a small safety margin inside each frame so the character is not clipped
- Keep the character centered and in a consistent position across all frames

### Animation continuity

- Each row is one animation, playing left to right in a loop
- Adjacent frames within a row must be smooth — no sudden jumps in position, pose, or expression
- Row 1 (idle) should have subtle breathing and blinking — do not make it completely static
- Row 6 (rest) should only show closed-eye breathing — no yawning frames

### Row reference

| Row | Animation | Triggered when | Notes |
|-----|-----------|---------------|-------|
| 1 | Idle / Peek | Default state | Gentle breathing, peeking from the bottom edge, occasional blink |
| 2 | Happy / Celebrate | Task completed | Cheerful expression and celebratory movement |
| 3 | Working / Focus | Pomodoro in progress | Focused expression and working posture |
| 4 | Thinking | AI processing a request | Thoughtful expression and pose |
| 5 | Reminder / Notify | Task due, plugin notifications | Alert expression and movement |
| 6 | Sleepy / Rest | Break state | Tired expression, closed-eye breathing |
| 7 | Dragging | Being dragged | Surprised or cooperative expression |

## manifest.json Fields

If you need to adjust the config manually, here are the main fields:

```json
{
  "id": "my-sprite",
  "name": "My Sprite",
  "description": "A description",
  "image": "sprite.png",
  "layout": {
    "columns": 8,
    "rows": 7
  },
  "scale": 0.40,
  "frameRate": 6,
  "chromaKey": {
    "targetColor": [255, 0, 255],
    "threshold": 100,
    "softness": 80,
    "spillSuppression": {
      "enabled": true,
      "threshold": 260,
      "strength": 0.90
    },
    "pixelArt": false
  }
}
```

| Field | Description |
|-------|-------------|
| `scale` | Display scale, controls the character size on desktop |
| `frameRate` | Animation frame rate, 5–8 is a good range |
| `chromaKey.targetColor` | Background color, always magenta `[255, 0, 255]` |
| `chromaKey.threshold` | Keying threshold, higher values remove more background |
| `chromaKey.softness` | Edge softness |
| `chromaKey.spillSuppression` | Suppresses magenta fringing on character edges |
| `pixelArt` | Pixel art mode, enable for pixel-style characters |

## Practical Tips

- Start with a frame rate of 6 and adjust — too fast feels jittery, too slow feels laggy
- Test the chroma key against the desktop background colors you use most
- Check the result at the actual desktop display size, not just at source resolution
