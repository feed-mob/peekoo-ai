# Peekoo AI - Issue Tracker (UI & Display)

This document tracks unresolved and resolved issues related to user interface, visual aesthetics, and display behavior.

## 🔴 Open Issues
| ID | Type | Description | Priority | Created |
|:---|:---|:---|:---|:---|
| ISS-001 | Feature | [Example] Add more micro-animations to Task list | Low | 2026-03-20 |
| ISS-002 | Bug | Health Reminders UI (v2) fails to load status data | High | 2026-03-20 |
| ISS-003 | Bug | Sprite switching in Settings causes display cut-off or layout shift | Medium | 2026-03-20 |
| ISS-004 | Bug | Pet shifts position on screen when opening menu or showing bubbles | Medium | 2026-03-20 |

### ISS-002: Health Reminders UI (v2) Data Loading Failure
- **Symptoms**: UI loads updated "v2" assets from global plugin directory, but tool calls (e.g., `health_get_status`) fail with "Failed to load health status" in JS.
- **Investigated**:
    - Fixed naming conventions for WASM exports (double-prefix issue).
    - Force-enabled plugin in SQLite database.
    - Verified cross-directory synchronization to `AppData\Local\Peekoo\peekoo\plugins`.
- **Blocked**: Plugin host appears to successfully discover but fails to properly route or execute the WASM tools, despite the HTML/JS being correctly served from the same directory.

### ISS-003: Sprite Switching Display/Layout Glitch
- **Symptoms**: Switching the pet sprite in Settings sometimes results in the sprite being partially cut off, or the relative position of the countdown badge/menu changing incorrectly.
- **Potential Cause**: `SpriteAnimation.tsx` or `SpritePeekBadge.tsx` might be using hardcoded offsets that don't account for varying scales or pivot points in different sprite manifests.

### ISS-004: Window Shifting During UI Transitions
- **Symptoms**: When right-clicking to expand the menu or when a `SpriteBubble` (speech bubble) appears, the pet character appears to "jump" or shift its absolute position on the screen.
- **Potential Cause**: The `resize_sprite_window` logic in `SpriteView.tsx` and the corresponding Rust command in `desktop-tauri/src/lib.rs` might have a race condition or DPI scaling issue when calculating the `deltaTop` to keep the window bottom-aligned.

## 🟡 In Progress
| ID | Type | Description | Assigned | Start Date |
|:---|:---|:---|:---|:---|
| - | - | - | - | - |

## 🟢 Resolved Issues
| ID | Type | Description | Resolution | Date |
|:---|:---|:---|:---|:---|
| RES-001 | UI | Pomodoro window too long with bottom padding | Adjusted default height to 380px and compacted layout. | 2026-03-20 |
| RES-002 | UX | Panel window sizes reset on restart | Implemented `localStorage` persistence in `use-panel-windows.ts`. | 2026-03-20 |
| RES-003 | UI | Pomodoro timer icon/controls too small/crowded | Fine-tuned radius to 70 and adjusted margins to fill 380px height. | 2026-03-20 |
| RES-004 | Bug | Focus Memo window didn't pop up after focus | Fixed missing `url` in `WebviewWindow` constructor. | 2026-03-20 |
| RES-005 | UI | Memo window corners had white artifacts | Removed OS decorations and enabled transparency for Memo. | 2026-03-20 |
| RES-006 | Style | Memo window lacked color harmony and light mode | Balanced colors (Brown/Mint and Green/Cream) with adaptive tokens. | 2026-03-20 |
| RES-007 | Style | Memo window transparency was too high/dull | Tuned transparency to 75-80% for better glass integration. | 2026-03-20 |

---
**Usage Guide**: 
- Use `ISS-XXX` for new issues.
- Move to `🟡 In Progress` when work starts.
- Move to `🟢 Resolved Issues` when closed.
