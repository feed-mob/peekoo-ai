# Changelog: Clean and Playful UI Overhaul

**Date:** 2026-03-10
**Author:** opencode

## Summary
Replaced the generic "Deep Space" AI-generated aesthetic with a custom, polished, and "Clean and Playful" design language across the desktop UI and plugin system.

## Changes

### 🎨 Global Theming (`apps/desktop-ui/src/index.css`)
- **Softened Color Palette**: Migrated from harsh `#0a0a0f` to a soft slate-indigo dark theme using OKLCH color space.
- **Improved Accents**: Brightened glow colors to vibrant pastels (Friendly Blue, Playful Purple, Gentle Cyan, Warm Pink).
- **Glassmorphism**: Upgraded panel backgrounds to `backdrop-blur-2xl` with softer, more translucent borders.
- **Bouncy Radii**: Increased global border radius for panels (`1.5rem`) and buttons (`1rem`) to create a friendlier "squircle" look.
- **Typography**: Added "Nunito" to the font stack for a rounder, more playful feel.

### ✨ Interactive Components (`apps/desktop-ui/src/components/ui/`)
- **Bouncy Buttons**: Refactored `button.tsx` with `framer-motion` to include physical scale reactions on hover (`1.02`) and tap (`0.96`) with spring physics.
- **Soft Inputs**: Updated `input.tsx` with rounded corners, `space-surface` background, and animated focus rings.
- **Custom Checkboxes**: Enhanced `checkbox.tsx` with thicker borders, larger tap targets, and smooth state transitions.
- **Pill Badges**: Transformed `badge.tsx` from rounded rectangles to full pill shapes.

### 🪟 Layout & Features
- **Panel Shell**: Redesigned `PanelShell.tsx` header for a cleaner look, removing the harsh bottom border and adding a bouncy close button.
- **Animated Chat**: Added spring entrance animations to `ChatMessage.tsx` and rounded chat bubbles with asymmetrical corners for better character.
- **Elevated Tasks**: Updated `TaskItem.tsx` with hover elevation, subtle shadows, and a clean fade-in delete action to reduce visual noise.

### 🔌 Plugin Alignment
- **Health Reminders**: Manually aligned `plugins/health-reminders/ui/panel.css` with the new design tokens (background, radiuses, and accents) to ensure visual consistency across the ecosystem.

## Technical Details
- **Libraries used**: `framer-motion` for spring physics and interactive scaling.
- **Styling**: Tailwind CSS v4 OKLCH color functions.
- **Transition types**: Replaced instant CSS transitions with `spring` transitions (`stiffness: 400, damping: 25`) for a more organic feel.
