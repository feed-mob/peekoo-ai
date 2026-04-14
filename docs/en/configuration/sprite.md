# Sprite Configuration

## What a Sprite Is

A sprite is the desktop character Peekoo renders in the transparent main window. Each sprite is self-contained in its own directory under `apps/desktop-ui/public/sprites/`.

## Directory Layout

```text
apps/desktop-ui/public/sprites/
└── [sprite-id]/
    ├── manifest.json
    └── sprite.png
```

## Manifest Fields

Each sprite provides a `manifest.json` with fields such as:

- `id`
- `name`
- `description`
- `image`
- `layout.columns`
- `layout.rows`
- `scale`
- `frameRate`
- `chromaKey`

These fields control how Peekoo loads, positions, animates, and keys the sprite on the desktop.

## Choosing or Preparing a Sprite

When you prepare sprite assets, focus on three things:

- a clean sprite sheet layout
- consistent frame positioning between animation frames
- a chroma key setup that removes the background cleanly

Peekoo expects a sprite sheet rather than a folder of separate frame images. That keeps animation loading simple and consistent.

## Window Behavior

Peekoo auto-resizes the main sprite window when UI chrome expands, for example when mini chat opens. The window remains non-resizable by default and only toggles resizability during programmatic resize. This helps preserve reliable click and drag behavior on Linux and Wayland.

## Adding a New Sprite

1. Create a new folder under `apps/desktop-ui/public/sprites/[new-id]`.
2. Add the sprite sheet image.
3. Add `manifest.json`.
4. Validate scale, chroma key, and frame layout.

## Practical Tips

- Keep the character centered between frames to avoid visible jumping.
- Test the sprite at the size Peekoo actually renders on the desktop, not only at source resolution.
- Use a frame rate that feels alive without becoming distracting.
- Check the sprite on the desktop background colors you use most often.

## Current Limitation

Sprite switching is still evolving. The repository already includes sprite metadata and active sprite settings support, but custom sprite workflows are still under active development.
